import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
  const { code } = (await request.json()) as { code: string };

  const tree = {
    name: 'Root',
    range: [0, code.length] as [number, number],
    type: 'Fake',
    children: [
      {
        name: `Length = ${code.length}`,
        range: [0, code.length] as [number, number],
        children: []
      }
    ]
  };

  return new Response(
    JSON.stringify({
      diagnostics: [],
      tree
    }),
    {
      status: 200,
      headers: { 'Content-Type': 'application/json' }
    }
  );
};
