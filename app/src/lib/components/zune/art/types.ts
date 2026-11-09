// Shared types for the Zune illustrations. Each model is one module in this
// folder (see README.md) that exports `model` and `render`.

export interface Finish {
	/** Stable id, lowercase, e.g. 'black', 'platinum'. */
	id: string;
	/** Display name, lowercase, e.g. 'black', 'platinum'. */
	name: string;
	/** Where this colorway is documented (a URL). */
	source: string;
	/** Optional note: limited edition, region, year. */
	note?: string;
}

export interface ZuneModel {
	/** 'zune30' | 'zuneflash' | 'zunehdd' | 'zunehd' */
	id: string;
	/** e.g. 'Zune 30' or 'Zune 4 / 8 / 16'. */
	name: string;
	year: number;
	/** Body width / height, from the real dimensions. */
	aspect: number;
	/** Real body size in mm (width, height, depth), for reference. */
	mm: [number, number, number];
	finishes: Finish[];
}

export interface ArtOptions {
	/** A Finish id from `model.finishes`. Unknown ids fall back to the first. */
	finish: string;
	/** Screen on (shows content) or off (dark glass). */
	lit: boolean;
	/** Rendered height in CSS px; width follows `model.aspect`. */
	height: number;
	/** Prefix for gradient/clip ids, unique per instance on a page. */
	uid: string;
	/** Clock shown on a lit screen. */
	now: Date;
}
