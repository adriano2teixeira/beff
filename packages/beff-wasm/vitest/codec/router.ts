import { Ctx } from "@beff/hono";
import * as E from "fp-ts/lib/Either";
import either2 from "./either2";
export default {
  ...either2,
  "/either": {
    post: async (
      _c: Ctx,
      b: {
        a: E.Either<string, number>;
      }
    ): Promise<E.Either<string, number>> => {
      return b.a;
    },
  },
  ["/date"]: {
    post: async (
      _c: Ctx,
      b: {
        a: Date;
      }
    ): Promise<Date> => {
      return b.a;
    },
    get: async (_c: Ctx, a: Date): Promise<Date> => {
      return a;
    },
  },
  "/bigint": {
    post: async (
      _c: Ctx,
      b: {
        a: bigint;
      }
    ): Promise<bigint> => {
      return b.a;
    },
    get: async (_c: Ctx, a: 1n): Promise<1n> => {
      return a;
    },
  },
  "/nan": {
    post: async (_c: Ctx, a: number): Promise<number> => {
      return a;
    },
  },
  "/nan2": {
    post: async (_c: Ctx, body: [number]): Promise<[number]> => {
      return body;
    },
  },
  ["/intersection"]: {
    post: async (
      _c: Ctx,
      p: {
        a: string;
      } & {
        b: number;
      }
    ): Promise<
      {
        a: string;
      } & {
        b: number;
      }
    > => {
      return p;
    },
    get: async (_c: Ctx, p: ("a" | "b") & "a"): Promise<("a" | "b") & "a"> => {
      return p;
    },
  },
  "/tuple1": {
    post: async (_c: Ctx, b: [number, number, ...string[]]): Promise<[number, number, ...string[]]> => {
      return b;
    },
  },
  "/tuple2": {
    post: async (_c: Ctx, b: [number, number]): Promise<[number, number]> => {
      return b;
    },
  },
  "/undefined": {
    post: (_c: Ctx, a: undefined): undefined => a,
  },
  "/union": {
    post: (_c: Ctx, a: "a" | "b"): "a" | "b" | "c" => a,
  },
  "/any_array": {
    post: (_c: Ctx, a: Array<any>): Array<any> => a,
  },
};
