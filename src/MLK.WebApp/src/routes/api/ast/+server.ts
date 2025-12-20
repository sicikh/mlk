import type { RequestHandler } from './$types';
import { buildAstFromSource } from '$lib/language/generated/AstApi.js';

export const POST: RequestHandler = async ({ request }) => {
  const { code } = (await request.json()) as { code: string };
  const result = buildAstFromSource(code);

  return new Response(JSON.stringify(result), {
    status: 200,
    headers: { 'Content-Type': 'application/json' }
  });
};
