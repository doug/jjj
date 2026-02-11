import starlightPlugin from '@astrojs/starlight-tailwind';

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
  theme: {
    extend: {
      colors: {
        background: '#FDFBF7',
        surface: '#F7F5F0',
        border: '#E5E2DC',
        'text-primary': '#2D2A26',
        'text-secondary': '#6B6660',
        accent: {
          DEFAULT: '#D946EF',
          hover: '#E879F9',
          low: '#FDF4FF',
          high: '#A21CAF',
        },
        success: '#6B9080',
        info: '#5B8A8A',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['Geist Mono', 'JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [starlightPlugin()],
};
