import 'pretendard/dist/web/variable/pretendardvariable.css';
import '@fontsource/jetbrains-mono/400.css';
import '@fontsource/jetbrains-mono/700.css';
import '@fontsource/space-grotesk/500.css';
import './app.css';
import App from './App.svelte';
import { mount } from 'svelte';
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';

async function ensureNotifications() {
  if (!(await isPermissionGranted())) { await requestPermission(); }
}
ensureNotifications();

const app = mount(App, { target: document.getElementById('app')! });
export default app;
