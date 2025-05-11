d:
  pnpm d

b:
  pnpm t:build

icon:
  pnpm tauri icon

db-migrate:
  cd src-tauri && cargo sqlx migrate run

db-prepare:
  cd src-tauri && cargo sqlx prepare -- --all-targets --all-features
