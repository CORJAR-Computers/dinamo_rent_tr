// backup.ts — Comandos y tipos para copias de seguridad y restauración
import { invokeCmd } from './base';

/** Una copia de seguridad en disco (services/backup.rs) */
export interface InfoCopiaBackup {
	nombre: string;
	tamanoBytes: number;
	modificado: string;
	cifrado: boolean;
}

/** Estado en memoria de los backups (config + última corrida + copias) */
export interface InfoBackup {
	directorio: string;
	maxCopies: number;
	horarios: string[];
	cifrado: boolean;
	ejecutando: boolean;
	ultimoBackup: string | null;
	ultimoResultado: string | null;
	ultimoError: string | null;
	proximaCorrida: string | null;
	copias: InfoCopiaBackup[];
	ultimaRestauracion: string | null;
	ultimaRestauracionError: string | null;
}

export const backupApi = {
	estado: (sessionId: string) => invokeCmd<InfoBackup>('backup_estado', { sessionId }),
	ahora: (sessionId: string) => invokeCmd<InfoBackup>('backup_ahora', { sessionId }),
	restaurar: (sessionId: string, archivo: string, password: string | null) =>
		invokeCmd<InfoBackup>('backup_restaurar', { sessionId, archivo, password })
};
