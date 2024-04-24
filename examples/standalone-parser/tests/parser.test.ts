import { it, expect } from "vitest";
import {
  PartialObject,
  AccessLevel,
  Extra,
  User,
  Repro1,
  SettingsUpdate,
  PartialSettings,
  LevelAndDSettings,
  OmitSettings,
  RequiredPartialObject,
  DiscriminatedUnion,
  DiscriminatedUnion2,
} from "../src/parser";

it("DiscriminatedUnion", () => {
  const validD: DiscriminatedUnion2 = {
    type: "d",
    valueD: 1,
  };
  expect(DiscriminatedUnion2.parse(validD)).toMatchInlineSnapshot(`
    {
      "type": "d",
      "valueD": 1,
    }
  `);
  const validD2: DiscriminatedUnion2 = {
    valueD: 1,
  };
  expect(DiscriminatedUnion2.parse(validD2)).toMatchInlineSnapshot(`
    {
      "type": undefined,
      "valueD": 1,
    }
  `);
  const valid: DiscriminatedUnion = {
    type: "a",
    subType: "a1",
    a1: "a",
  };
  expect(DiscriminatedUnion.parse(valid)).toMatchInlineSnapshot(`
    {
      "a1": "a",
      "a11": undefined,
      "subType": "a1",
      "type": "a",
    }
  `);
  const valid3: DiscriminatedUnion = {
    type: "a",
    subType: "a1",
    a1: "a",
    a11: "a",
  };
  expect(DiscriminatedUnion.parse(valid3)).toMatchInlineSnapshot(`
    {
      "a1": "a",
      "a11": "a",
      "subType": "a1",
      "type": "a",
    }
  `);
  const invalid4 = {
    type: "a",
    subType: "a1",
    a1: "a",
    a11: 123,
  };
  expect(DiscriminatedUnion.safeParse(invalid4)).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "message": "expected string",
          "path": [
            "a11",
          ],
          "received": 123,
        },
      ],
      "success": false,
    }
  `);
  const valid2: DiscriminatedUnion = {
    type: "b",
    value: 1,
  };
  expect(DiscriminatedUnion.parse(valid2)).toMatchInlineSnapshot(`
    {
      "type": "b",
      "value": 1,
    }
  `);

  expect(
    DiscriminatedUnion.safeParse({
      type: "a",
    })
  ).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "message": "expected discriminator key \\"subType\\"",
          "path": [],
          "received": {
            "type": "a",
          },
        },
      ],
      "success": false,
    }
  `);
  expect(
    DiscriminatedUnion.safeParse({
      type: "c",
    })
  ).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "message": "expected one of \\"a\\", \\"b\\"",
          "path": [
            "type",
          ],
          "received": "c",
        },
      ],
      "success": false,
    }
  `);
  expect(DiscriminatedUnion.safeParse({})).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "message": "expected discriminator key \\"type\\"",
          "path": [],
          "received": {},
        },
      ],
      "success": false,
    }
  `);
});
it("repro1", () => {
  expect(Repro1.parse({})).toMatchInlineSnapshot(`
    {
      "sizes": undefined,
    }
  `);
});
it("PartialObject", () => {
  expect(PartialObject.parse({})).toMatchInlineSnapshot(`
    {
      "a": undefined,
      "b": undefined,
    }
  `);
});
it("PartialSettings", () => {
  expect(PartialSettings.parse({})).toMatchInlineSnapshot(`
    {
      "a": undefined,
      "d": undefined,
      "level": undefined,
    }
  `);
});
it("LevelAndDSettings", () => {
  const valid: LevelAndDSettings = {
    level: "a",
    d: {
      tag: "d",
    },
  };
  expect(LevelAndDSettings.parse(valid)).toMatchInlineSnapshot(`
    {
      "d": {
        "tag": "d",
      },
      "level": "a",
    }
  `);
});
it("OmitSettings", () => {
  const valid: OmitSettings = {
    level: "a",
    d: {
      tag: "d",
    },
  };
  expect(OmitSettings.parse(valid)).toMatchInlineSnapshot(`
    {
      "d": {
        "tag": "d",
      },
      "level": "a",
    }
  `);
});
it("RequiredPartialObject", () => {
  const valid: RequiredPartialObject = {
    a: "a",
    b: 1,
  };
  expect(RequiredPartialObject.parse(valid)).toMatchInlineSnapshot(`
    {
      "a": "a",
      "b": 1,
    }
  `);
});
it("OneOfSettingsUpdate", () => {
  const valid: SettingsUpdate = {
    tag: "d",
  };
  expect(SettingsUpdate.parse(valid)).toMatchInlineSnapshot(`
    {
      "tag": "d",
    }
  `);
});

it("checks records", () => {
  expect(Extra.safeParse({ key: 123 })).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "message": "expected string",
          "path": [
            "key",
          ],
          "received": 123,
        },
      ],
      "success": false,
    }
  `);
});

it("works on recursive type", () => {
  const valid: User = {
    name: "User1",
    friends: [
      {
        name: "User2",
        friends: [],
        accessLevel: AccessLevel.USER,
        avatarSize: "100x100",
        extra: {},
      },
    ],
    accessLevel: AccessLevel.ADMIN,
    avatarSize: "100x100",
    extra: {
      key: "value",
    },
  };
  expect(User.parse(valid)).toMatchInlineSnapshot(`
    {
      "accessLevel": "ADMIN",
      "avatarSize": "100x100",
      "extra": {
        "key": "value",
      },
      "friends": [
        {
          "accessLevel": "USER",
          "avatarSize": "100x100",
          "extra": {},
          "friends": [],
          "name": "User2",
        },
      ],
      "name": "User1",
    }
  `);
  const invalid = {
    name: "User1",
    friends: [
      {
        name: "User2",
      },
    ],
  };
  expect(User.safeParse(invalid)).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "message": "expected one of \\"ADMIN\\", \\"USER\\"",
          "path": [
            "accessLevel",
          ],
          "received": undefined,
        },
        {
          "message": "expected string",
          "path": [
            "avatarSize",
          ],
          "received": undefined,
        },
        {
          "message": "expected object",
          "path": [
            "extra",
          ],
          "received": undefined,
        },
        {
          "message": "expected one of \\"ADMIN\\", \\"USER\\"",
          "path": [
            "friends",
            "[0]",
            "accessLevel",
          ],
          "received": undefined,
        },
        {
          "message": "expected string",
          "path": [
            "friends",
            "[0]",
            "avatarSize",
          ],
          "received": undefined,
        },
        {
          "message": "expected object",
          "path": [
            "friends",
            "[0]",
            "extra",
          ],
          "received": undefined,
        },
        {
          "message": "expected array",
          "path": [
            "friends",
            "[0]",
            "friends",
          ],
          "received": undefined,
        },
      ],
      "success": false,
    }
  `);
});
