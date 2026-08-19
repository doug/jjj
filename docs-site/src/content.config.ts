import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

export const collections = {
	docs: defineCollection({
		loader: glob({
			base: '../docs',
			// audit/ and plans/ are internal working documents — findings logs and
			// completed implementation plans — not user documentation. They carry no
			// Starlight frontmatter, so including them fails the whole build (which
			// is what silently broke the docs deploy from 2026-05-30 onward).
			pattern: ['**/!(_)*.{md,mdx}', '!plans/**', '!audit/**'],
		}),
		schema: docsSchema(),
	}),
};
