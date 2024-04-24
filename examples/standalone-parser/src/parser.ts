import parse from "./generated/parser";

export enum AccessLevel {
  ADMIN = "ADMIN",
  USER = "USER",
}

type AvatarSize = `${number}x${number}`;
export type Extra = Record<string, string>;

export type User = {
  accessLevel: AccessLevel;
  name: string;
  friends: User[];
  avatarSize: AvatarSize;
  extra: Extra;
};

export type PublicUser = Omit<User, "friends">;

type WithOptionals = {
  optional?: string;
};

type Req = Required<WithOptionals>;

export interface Repro2 {
  useSmallerSizes: boolean;
}

export interface Repro1 {
  sizes?: Repro2;
}

export type Settings = {
  a: string;
  level: "a" | "b";
  d: {
    tag: "d";
  };
};

export type SettingsUpdate = Settings["a" | "level" | "d"];
export type Mapped = {
  [K in "a" | "b"]: {
    value: K;
  };
};
export type MappedOptional = {
  [K in "a" | "b"]?: {
    value: K;
  };
};

export type PartialSettings = Partial<Settings>;

export type LevelAndDSettings = Pick<Settings, "level" | "d">;

export type OmitSettings = Omit<Settings, "a">;

export type PartialObject = {
  a?: string;
  b?: number;
};

export type RequiredPartialObject = Required<PartialObject>;

export type DiscriminatedUnion =
  | {
      type: "a";
      subType: "a1";
      a1: string;
      a11?: string;
    }
  | {
      type: "a";
      subType: "a2";
      a2: string;
    }
  | {
      type: "b";
      value: number;
    };
export type DiscriminatedUnion2 =
  | {
      type: "a";
      subType: "a1";
      a1: string;
      a11?: string;
    }
  | {
      type: "a";
      subType: "a2";
      a2: string;
    }
  | {
      type?: "d";
      valueD: number;
    }
  | {
      type: "b";
      value: number;
    };

export const {
  RequiredPartialObject,
  LevelAndDSettings,
  PartialSettings,
  Extra,
  User,
  PublicUser,
  Repro1,
  SettingsUpdate,
  Mapped,
  MappedOptional,
  OmitSettings,
  PartialObject,
  DiscriminatedUnion,
  DiscriminatedUnion2,
} = parse.buildParsers<{
  OmitSettings: OmitSettings;
  RequiredPartialObject: RequiredPartialObject;
  LevelAndDSettings: LevelAndDSettings;
  PartialSettings: PartialSettings;
  Extra: Extra;
  User: User;
  PublicUser: PublicUser;
  Req: Req;
  Repro1: Repro1;
  SettingsUpdate: SettingsUpdate;
  Mapped: Mapped;
  MappedOptional: MappedOptional;
  PartialObject: PartialObject;
  DiscriminatedUnion: DiscriminatedUnion;
  DiscriminatedUnion2: DiscriminatedUnion2;
}>();
