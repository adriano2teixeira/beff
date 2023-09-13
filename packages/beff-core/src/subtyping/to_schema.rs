use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use crate::{
    ast::{
        json::Json,
        json_schema::{JsonSchema, Optionality},
    },
    open_api_ast::Validator,
};

use super::{
    bdd::{Atom, Bdd, ListAtomic, MappingAtomic},
    semtype::{SemType, SemTypeContext},
    subtype::{ProperSubtype, StringLitOrFormat, SubTypeTag},
};

pub enum SchemaMemo {
    Schema(JsonSchema),
    Undefined(String),
}
struct SchemerContext<'a> {
    ctx: &'a SemTypeContext,

    schemer_memo: BTreeMap<Rc<SemType>, SchemaMemo>,
    validators: Vec<Validator>,

    recursive_validators: BTreeSet<String>,

    counter: usize,
}

impl<'a> SchemerContext<'a> {
    fn new(ctx: &'a SemTypeContext) -> Self {
        Self {
            ctx: ctx,
            validators: vec![],
            schemer_memo: BTreeMap::new(),
            counter: 0,
            recursive_validators: BTreeSet::new(),
        }
    }

    fn mapping_atom_schema(&mut self, mt: &Rc<MappingAtomic>) -> JsonSchema {
        let mut acc: Vec<(String, Optionality<JsonSchema>)> = vec![];

        for (k, v) in mt.iter() {
            let schema = self.to_schema(v, None);
            let ty = if v.has_void() {
                schema.optional()
            } else {
                schema.required()
            };
            acc.push((k.clone(), ty));
        }

        JsonSchema::Object(BTreeMap::from_iter(acc))
    }

    fn to_schema_mapping_node(
        &mut self,
        atom: &Rc<Atom>,
        left: &Rc<Bdd>,
        middle: &Rc<Bdd>,
        right: &Rc<Bdd>,
    ) -> JsonSchema {
        let mt = match atom.as_ref() {
            Atom::Mapping(a) => self.ctx.get_mapping_atomic(*a).clone(),
            _ => unreachable!(),
        };

        let explained_sts = self.mapping_atom_schema(&mt);

        let mut acc = vec![];

        match left.as_ref() {
            Bdd::True => {
                acc.push(explained_sts.clone());
            }
            Bdd::False => {
                // noop
            }
            Bdd::Node {
                atom,
                left,
                middle,
                right,
            } => {
                let ty = vec![explained_sts.clone()]
                    .into_iter()
                    .chain(vec![self.to_schema_mapping_node(atom, left, middle, right)]);
                acc.push(JsonSchema::AllOf(BTreeSet::from_iter(ty)));
            }
        };

        match middle.as_ref() {
            Bdd::False => {
                // noop
            }
            Bdd::True | Bdd::Node { .. } => {
                acc.push(self.to_schema_mapping(middle));
            }
        }
        match right.as_ref() {
            Bdd::True => {
                acc.push(JsonSchema::StNot(Box::new(explained_sts)));
            }
            Bdd::False => {
                // noop
            }
            Bdd::Node {
                atom,
                left,
                middle,
                right,
            } => {
                let ty = JsonSchema::AllOf(BTreeSet::from_iter(vec![
                    JsonSchema::StNot(Box::new(explained_sts)),
                    self.to_schema_mapping_node(atom, left, middle, right),
                ]));
                acc.push(ty)
            }
        }
        return JsonSchema::any_of(acc);
    }

    fn to_schema_mapping(&mut self, bdd: &Rc<Bdd>) -> JsonSchema {
        match bdd.as_ref() {
            Bdd::True => todo!(),
            Bdd::False => todo!(),
            Bdd::Node {
                atom,
                left,
                middle,
                right,
            } => self.to_schema_mapping_node(atom, left, middle, right),
        }
    }

    fn list_atom_schema(&mut self, mt: &Rc<ListAtomic>) -> JsonSchema {
        if mt.prefix_items.is_empty() {
            return JsonSchema::Array(Box::new(self.to_schema(&mt.items, None)));
        }

        let prefix_items = mt
            .prefix_items
            .iter()
            .map(|it| self.to_schema(it, None))
            .collect();

        let items = if mt.items.is_never() {
            None
        } else {
            Some(Box::new(self.to_schema(&mt.items, None)))
        };
        return JsonSchema::Tuple {
            prefix_items,
            items,
        };
    }

    fn to_schema_list_node(
        &mut self,
        atom: &Rc<Atom>,
        left: &Rc<Bdd>,
        middle: &Rc<Bdd>,
        right: &Rc<Bdd>,
    ) -> JsonSchema {
        let lt = match atom.as_ref() {
            Atom::List(a) => self.ctx.get_list_atomic(*a).clone(),
            _ => unreachable!(),
        };

        let explained_sts = self.list_atom_schema(&lt);

        let mut acc = vec![];

        match left.as_ref() {
            Bdd::True => {
                acc.push(explained_sts.clone());
            }
            Bdd::False => {
                // noop
            }
            Bdd::Node {
                atom,
                left,
                middle,
                right,
            } => {
                let ty = vec![explained_sts.clone()]
                    .into_iter()
                    .chain(vec![self.to_schema_list_node(atom, left, middle, right)]);
                acc.push(JsonSchema::AllOf(BTreeSet::from_iter(ty)));
            }
        };

        match middle.as_ref() {
            Bdd::False => {
                // noop
            }
            Bdd::True | Bdd::Node { .. } => {
                acc.push(self.to_schema_list(middle));
            }
        }
        match right.as_ref() {
            Bdd::True => {
                acc.push(JsonSchema::StNot(Box::new(explained_sts)));
            }
            Bdd::False => {
                // noop
            }
            Bdd::Node {
                atom,
                left,
                middle,
                right,
            } => {
                let ty = JsonSchema::AllOf(BTreeSet::from_iter(vec![
                    JsonSchema::StNot(Box::new(explained_sts)),
                    self.to_schema_list_node(atom, left, middle, right),
                ]));
                acc.push(ty)
            }
        }
        return JsonSchema::any_of(acc);
    }

    fn to_schema_list(&mut self, bdd: &Rc<Bdd>) -> JsonSchema {
        match bdd.as_ref() {
            Bdd::True => todo!(),
            Bdd::False => todo!(),
            Bdd::Node {
                atom,
                left,
                middle,
                right,
            } => self.to_schema_list_node(atom, left, middle, right),
        }
    }

    fn maybe_not(it: JsonSchema, add_not: bool) -> JsonSchema {
        if add_not {
            JsonSchema::StNot(Box::new(it))
        } else {
            it
        }
    }

    fn to_schema_no_cache(&mut self, ty: &SemType) -> JsonSchema {
        if ty.all == 0 && ty.subtype_data.is_empty() {
            return JsonSchema::StNever;
        }

        let mut acc = BTreeSet::new();

        for t in SubTypeTag::all() {
            if (ty.all & t.code()) != 0 {
                match t {
                    SubTypeTag::Null => {
                        acc.insert(JsonSchema::Null);
                    }
                    SubTypeTag::Boolean => {
                        acc.insert(JsonSchema::Boolean);
                    }
                    SubTypeTag::Number => {
                        acc.insert(JsonSchema::Number);
                    }
                    SubTypeTag::String => {
                        acc.insert(JsonSchema::String);
                    }
                    SubTypeTag::Void => {
                        // noop
                    }
                    SubTypeTag::Mapping => {
                        acc.insert(JsonSchema::AnyObject);
                    }
                    SubTypeTag::List => {
                        acc.insert(JsonSchema::AnyArrayLike);
                    }
                    SubTypeTag::Any => unreachable!("we do not allow creation of any"),
                };
            }
        }

        for s in &ty.subtype_data {
            match s.as_ref() {
                ProperSubtype::Boolean(v) => {
                    acc.insert(JsonSchema::Const(Json::Bool(*v)));
                }
                ProperSubtype::Number { allowed, values } => {
                    for h in values {
                        acc.insert(Self::maybe_not(
                            JsonSchema::Const(Json::Number(h.clone())),
                            !allowed,
                        ));
                    }
                }
                ProperSubtype::String { allowed, values } => {
                    for h in values {
                        match h {
                            StringLitOrFormat::Lit(st) => {
                                acc.insert(Self::maybe_not(
                                    JsonSchema::Const(Json::String(st.clone())),
                                    !allowed,
                                ));
                            }
                            StringLitOrFormat::Format(fmt) => {
                                acc.insert(Self::maybe_not(
                                    JsonSchema::StringWithFormat(fmt.clone()),
                                    !allowed,
                                ));
                            }
                        }
                    }
                }
                ProperSubtype::Mapping(bdd) => {
                    acc.insert(self.to_schema_mapping(bdd));
                }
                ProperSubtype::List(bdd) => {
                    acc.insert(self.to_schema_list(bdd));
                }
            };
        }

        JsonSchema::AnyOf(acc)
    }
    pub fn to_schema(&mut self, ty: &Rc<SemType>, name: Option<&str>) -> JsonSchema {
        let new_name = match name {
            Some(n) => n.to_string(),
            None => {
                self.counter = self.counter + 1;
                format!("t_{}", self.counter)
            }
        };
        if let Some(mater) = self.schemer_memo.get(ty) {
            match mater {
                SchemaMemo::Schema(mater) => return mater.clone(),
                SchemaMemo::Undefined(ref_name) => {
                    self.recursive_validators.insert(ref_name.clone());
                    return JsonSchema::Ref(ref_name.into());
                }
            }
        } else {
            self.schemer_memo
                .insert(ty.clone(), SchemaMemo::Undefined(new_name.clone()));
        }
        let schema = self.to_schema_no_cache(ty);
        self.schemer_memo
            .insert(ty.clone(), SchemaMemo::Schema(schema.clone()));
        self.validators.push(Validator {
            name: new_name,
            schema: schema.clone(),
        });

        schema
    }
}

pub fn to_validators(ctx: &SemTypeContext, ty: &Rc<SemType>, name: &str) -> Vec<Validator> {
    let mut schemer = SchemerContext::new(ctx);
    let out = schemer.to_schema(ty, Some(name));
    let vs = schemer
        .validators
        .into_iter()
        .filter(|it| schemer.recursive_validators.contains(&it.name));

    if schemer.recursive_validators.contains(name) {
        vs.collect()
    } else {
        vs.chain(vec![Validator {
            name: name.into(),
            schema: out,
        }])
        .collect()
    }
}
