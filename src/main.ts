import './app.css';
import { mount } from 'svelte';
import App from './routes/+page.svelte';

const app = mount(App, {
  target: document.getElementById('app')!,
});

export default app;
