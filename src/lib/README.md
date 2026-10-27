# Shared frontend modules

[Startup animation and replay](./startup/README.md) owns pure launch policy,
elapsed-time motion, and the shared Svelte visual. Routes mount these modules;
the hidden root route owns real updater work, the passive intro route renders
native-owned surfaces, and the preview injects only simulated updates.

Shared modules must not start sleep prevention or native updates on import.
