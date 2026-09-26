import './commands';

Cypress.on('uncaught:exception', (err) => {
  if (err.message.includes('__TAURI_INTERNALS__')) return false;
  if (err.message.includes('__TAURI_EVENT_PLUGIN_INTERNALS__')) return false;
  return true;
});
