import { docs } from 'collections/server';
import { loader } from 'fumadocs-core/source';

// The docs tree is mounted at the site root so the "Overview" page is served
// at route "/". Generated `.source` output is consumed via the `collections/*`
// path alias (see tsconfig.json).
export const source = loader({
  baseUrl: '/',
  source: docs.toFumadocsSource(),
});
