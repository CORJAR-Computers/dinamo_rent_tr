## 📋 Descripción

<!-- Breve descripción de qué hace este PR y por qué -->

## 🔗 Issue relacionado

<!-- Fixes #123, closes #456, o "N/A" -->

## 🧪 Tipo de cambio

- [ ] Bug fix (cambio que no rompe nada)
- [ ] Nueva funcionalidad
- [ ] Breaking change (cambio que requiere migración)
- [ ] Refactor
- [ ] Documentación
- [ ] CI/CD

## ✅ Checklist

- [ ] `bun run lint` pasa sin errores
- [ ] `bun run check` pasa sin errores (svelte-check 0/0)
- [ ] `bun run test` pasa (vitest)
- [ ] `cargo test` pasa (incl. integración con BD sembrada)
- [ ] `cargo clippy -- -D warnings` sin warnings
- [ ] `cargo fmt --check` pasa
- [ ] Si hay migraciones: son idempotentes y tienen `EXECUTE BLOCK` con guards
- [ ] Si hay cambios de UI: responsive y accessible (aria-label, focus-visible)
- [ ] Si hay cambios de seguridad: revisado por SECURITY.md

## 📸 Capturas / evidencia

<!-- Si hay cambios de UI, añade capturas antes/después -->

## 📝 Notas para el reviewer

<!-- Cualquier contexto adicional, decisiones de diseño, o áreas de riesgo -->
