import '../styles.css';
import { MindVaultAdmin } from './admin.js';

function boot() {
  const admin = new MindVaultAdmin();
  window.mindVaultAdmin = admin;
  admin.init();
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', boot);
} else {
  boot();
}
