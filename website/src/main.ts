import { mount } from 'svelte';
import '@fontsource/geist/latin-400.css';
import '@fontsource/geist/latin-500.css';
import '@fontsource/geist/latin-600.css';
import '@fontsource/geist/latin-700.css';
import '@fontsource/geist-mono/latin-400.css';
import '@fontsource/geist-mono/latin-500.css';
import './site.css';
import App from './App.svelte';

void import('@fontsource/noto-serif-sc/600.css');

const target = document.getElementById('app');
if (target) {
  mount(App, { target });
}
