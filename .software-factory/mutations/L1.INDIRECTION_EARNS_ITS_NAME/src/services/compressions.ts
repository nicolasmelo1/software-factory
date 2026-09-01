import { insertCompression as insertCompressionDb, queryCompressionRows } from "@db";

export async function insertCompression(data: CompressionInsertData) {
  return await insertCompressionDb(data);
}

export async function listCompressions(reportId: number) {
  return queryCompressionRows(reportId);
}
