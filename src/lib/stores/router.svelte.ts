// Simple client-side router using Svelte 5 runes
export type Route = 'home' | 'settings';

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
