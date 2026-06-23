import { source } from '@/lib/source';
import { createFromSource } from 'fumadocs-core/search/server';

// Default Fumadocs (Orama) static search index, built from the docs source.
export const { GET } = createFromSource(source, {
  language: 'english',
});
