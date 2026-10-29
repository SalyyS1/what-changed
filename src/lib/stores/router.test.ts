import { afterEach, describe, expect, it, vi } from 'vitest';
import { Router } from './router.svelte';
import { routeById } from '../../app/routes';

afterEach(() => { window.location.hash = ''; });

describe('owner workspace routes', () => {
  it('restores app and request parameters on deep links', () => {
    window.location.hash = '#/apps/app-a/change/change-a';
    const router = new Router();
    const detach = router.attach();
    expect(router.current.id).toBe('owner-change');
    expect(router.params).toEqual({ appId: 'app-a', changeId: 'change-a' });
    expect(router.isActive(routeById('owner-apps')!)).toBe(true);
    detach();
  });

  it('rejects malformed and unsafe encoded route parameters', () => {
    const router = new Router();
    const detach = router.attach();
    for (const hash of ['#/apps/%ZZ', '#/apps/app-a/change/%2Fsecret', '#/apps/%00']) {
      window.location.hash = hash;
      window.dispatchEvent(new HashChangeEvent('hashchange'));
      expect(router.current.id).toBe('home');
      expect(router.params).toEqual({});
    }
    detach();
  });

  it('guards explicit navigation before changing the URL', () => {
    window.location.hash = '#/apps/app-a/change/change-a';
    const router = new Router(); const detach = router.attach();
    const blocked = vi.fn(); const remove = router.setNavigationGuard(() => false, blocked);
    router.navigate(routeById('owner-app')!, window, { appId: 'app-b' });
    expect(window.location.hash).toBe('#/apps/app-a/change/change-a');
    expect(blocked).toHaveBeenCalledWith(routeById('owner-app'), { appId: 'app-b' });
    remove(); detach();
  });

  it('guards browser hash changes and retains the current request params', () => {
    window.location.hash = '#/apps/app-a/change/change-a';
    const router = new Router(); const detach = router.attach();
    const blocked = vi.fn(); router.setNavigationGuard((route) => route.id === 'owner-change', blocked);
    window.location.hash = '#/timeline';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(router.current.id).toBe('owner-change');
    expect(router.params.changeId).toBe('change-a');
    expect(window.location.hash).toBe('#/apps/app-a/change/change-a');
    expect(blocked).toHaveBeenCalled(); detach();
  });

  it('replaces a saved new-draft URL with its persisted request identifier', () => {
    window.location.hash = '#/apps/app-a/change';
    const router = new Router(); const detach = router.attach();
    router.replace(routeById('owner-change')!, { appId: 'app-a', changeId: 'saved-change' });
    expect(window.location.hash).toBe('#/apps/app-a/change/saved-change');
    expect(router.params.changeId).toBe('saved-change'); detach();
  });
});
