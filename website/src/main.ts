import { mount } from 'svelte';
import './site.css';
import App from './App.svelte';

const target = document.getElementById('app');
if (target) {
  mount(App, { target });
}
