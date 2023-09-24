import { beffLocalClient } from "@/beff/router-app";
import { NextResponse } from "next/server";

export async function GET(
  _request: Request,
  { params }: { params: { projectId: string; versionId: string } }
) {
  const version = await beffLocalClient["/project/{projectId}/version/{versionId}"].get(
    params.projectId,
    params.versionId
  );
  if (version == null) {
    throw new Error("Not found");
  }
  return new Response(version.schema, {
    status: 200,
    headers: {
      "content-type": "application/json",
    },
  });
}
