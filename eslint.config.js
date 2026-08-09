// ESLint flat config para dinamo_rent_tr (TAREA E6 del Grupo E).
//
// Requiere (añadidas a package.json devDependencies; el usuario debe instalarlas
// con `bun install` o `npm install`):
//   - eslint (^9.0.0, flat config por defecto)
//   - @eslint/js (^9.0.0)
//   - @typescript-eslint/parser (^8.0.0)
//   - @typescript-eslint/eslint-plugin (^8.0.0)
//   - eslint-plugin-svelte (^2.40.0)
//   - svelte-eslint-parser (^0.41.0)
//   - eslint-config-prettier (^9.0.0)
//   - prettier (^3.0.0)
//   - prettier-plugin-svelte (^3.0.0)
//
// Stack: ESLint 9 (flat config), TypeScript 6, Svelte 5.56, SvelteKit 2.
// Estilo del repo: indentación con tabs, comillas simples, punto y coma sí.
// (Ver .prettierrc.)

import js from '@eslint/js';
import tsParser from '@typescript-eslint/parser';
import tsPlugin from '@typescript-eslint/eslint-plugin';
import sveltePlugin from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import prettier from 'eslint-config-prettier';

export default [
	// Ignorados globales (no se lintean)
	{
		ignores: [
			'.svelte-kit/**',
			'build/**',
			'dist/**',
			'node_modules/**',
			'src-tauri/**', // Rust se valida con `cargo clippy`
			'src-tauri/resources/**',
			'scripts/**', // .mjs scripts de smoke test (CDP, no producción)
			'**/*.test.ts', // tests: reglas más relajadas (ver override abajo)
			'**/*.test.svelte'
		]
	},
	// Config base JS recomendada
	js.configs.recommended,
	// TypeScript recomendado (configurado manualmente con los paquetes individuales
	// que pide la tarea; equivalente a `...ts.configs.recommended` del paquete unificado)
	{
		files: ['**/*.ts', '**/*.svelte.ts'],
		languageOptions: {
			parser: tsParser,
			parserOptions: {
				sourceType: 'module',
				ecmaVersion: 'latest'
			}
		},
		plugins: {
			'@typescript-eslint': tsPlugin
		},
		rules: {
			...tsPlugin.configs.recommended.rules
		}
	},
	// Svelte: reglas de sintaxis y buenas prácticas
	...sveltePlugin.configs['flat/recommended'],
	// Asociar el parser de Svelte a los .svelte y usar el parser de TS dentro
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parser: svelteParser,
			parserOptions: {
				parser: tsParser,
				extraFileExtensions: ['.svelte']
			}
		}
	},
	{
		// Reglas custom del proyecto (TAREA E6)
		files: ['**/*.ts', '**/*.svelte', '**/*.svelte.ts'],
		rules: {
			// Sin `any` sin justificación. Si necesitas uno, usa `eslint-disable-next-line`
			// con un comentario explicando por qué. No es error para no romper el build.
			'@typescript-eslint/no-explicit-any': 'warn',
			// Sin @ts-ignore (usar @ts-expect-error con razón, o arreglar el tipo).
			// En este repo NO hay ninguno hoy; la regla evita que se introduzcan.
			'@typescript-eslint/ban-ts-comment': 'error',
			// Sin console.log en producción. Permitido warn/error/info y en tests.
			'no-console': ['warn', { allow: ['warn', 'error', 'info'] }],
			// `prefer-const` ya viene en recommended; lo dejamos.
			'@typescript-eslint/no-unused-vars': [
				'warn',
				{
					argsIgnorePattern: '^_',
					varsIgnorePattern: '^_',
					caughtErrorsIgnorePattern: '^_'
				}
			]
		}
	},
	{
		// Tests: permitir `any`, `console.log` y aserciones no-null.
		files: ['**/*.test.ts', '**/*.test.svelte', 'src/test/**'],
		rules: {
			'@typescript-eslint/no-explicit-any': 'off',
			'no-console': 'off',
			'@typescript-eslint/no-non-null-assertion': 'off',
			'@typescript-eslint/no-explicit-any': 'off'
		}
	},
	// Desactivar reglas que conflictúan con Prettier (al final para tener prioridad)
	prettier,
	...sveltePlugin.configs['flat/prettier']
];
