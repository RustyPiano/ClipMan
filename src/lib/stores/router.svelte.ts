import type { Route } from '$lib/types';
export type { Route } from '$lib/types';

// Simple client-side router using Svelte 5 runes

class Router {
  currentRoute = $state<Route>('home');

  navigate(route: Route) {
    this.currentRoute = route;
  }

  goHome() {
    this.navigate('home');
  }

  goToSettings() {
    this.navigate('settings');
  }
}

export const router = new Router();
