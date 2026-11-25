import { mount } from 'svelte';
import '../styles/tokens.css';
import '../styles/base.css';
import OverlayApp from './OverlayApp.svelte';

mount(OverlayApp, {
  target: document.getElementById('overlay-app')!,
});
