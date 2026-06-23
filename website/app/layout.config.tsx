// Re-export the shared base layout options for convenience.
// Kept separate from app/layout.tsx (the root HTML shell) so layouts can import
// nav/footer options without pulling in the root document.
export { baseOptions } from '@/lib/layout.shared';
