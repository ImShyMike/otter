import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import type { SearchResult } from './types';

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

function formatFloat(num: number, decimals: number): string {
	if (num === 0) return '0';
	return Number(num.toFixed(decimals)).toString();
}

export function formatApproved(ts: number | null): string {
	if (!ts) return '—';
	return new Date(ts * 1000).toLocaleDateString();
}

export function formatHours(r: SearchResult): string {
	if (r.hours != null) {
		if (r.true_hours != null) {
			if (Math.round(r.true_hours) === r.hours) {
				if (r.true_hours / 10 === 0) {
					return `${r.hours}h`;
				}
				return `${formatFloat(r.true_hours, 1)}h`;
			}

			return `${formatFloat(r.true_hours, 1)}h`;
		}

		return `${r.hours}h`;
	}

	if (r.true_hours != null) {
		return `~${r.true_hours.toFixed(1)}h`;
	}

	return '0';
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, 'child'> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, 'children'> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
