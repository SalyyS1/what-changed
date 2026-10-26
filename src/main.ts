import { mount } from 'svelte';
import App from './app/App.svelte';
import './styles/app.css';

const target = document.getElementById('app');
if (!target) {
  // index.html is ours and committed; a missing mount point is a build defect,
  // not a runtime condition to recover from.
  throw new Error('mount target #app is missing from index.html');
}

export default mount(App, { target });
