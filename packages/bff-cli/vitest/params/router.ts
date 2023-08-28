import { Cookie, Header } from "./bff-generated";
import { cors } from "hono/cors";
export default {
  ["USE/*"]: [cors()],
  ["GET/hello"]: async (): Promise<string> => {
    return "Hello!";
  },
  [`GET/path-param/{name}`]: async (name: string): Promise<string> => {
    return name;
  },

  ["GET/query-param"]: async (limit: number): Promise<number> => {
    return limit;
  },
  [`GET/header-param`]: async (user_agent: Header<string>): Promise<string> => {
    return user_agent;
  },
  [`GET/cookie-param`]: async (ads_ids: Cookie<string>): Promise<string> => {
    return ads_ids;
  },
  [`POST/hello`]: async (): Promise<string> => {
    return "Hello!";
  },
  [`POST/path-param/{name}`]: async (name: string): Promise<string> => {
    return name;
  },
  [`POST/req-body`]: async (data: { a: string }): Promise<string> => {
    return data.a;
  },
  [`GET/path-param-string/{name}`]: async (name: string): Promise<string> => {
    return name;
  },
  [`GET/path-param-number/{id}`]: async (id: number): Promise<number> => {
    return id;
  },
  [`GET/path-param-boolean/{flag}`]: async (
    flag: boolean
  ): Promise<boolean> => {
    return flag;
  },
  [`GET/path-param-union/{id}`]: async (id: ValidIds): Promise<ValidIds> => {
    return id;
  },
};
type ValidIds = 123 | 456;
