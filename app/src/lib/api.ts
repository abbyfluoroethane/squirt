// Data layer for the UI. In the Tauri app it calls the Rust backend; in a
// plain browser (`npm run dev`) it uses mock data so the UI can be developed
// and reviewed without a device.

export interface Usage {
	music: number;
	videos: number;
	pictures: number;
	other: number;
}

export interface DeviceStatus {
	connected: boolean;
	name: string;
	model: string;
	firmware: string;
	capacity: number;
	free: number;
	usage: Usage;
}

export interface Track {
	disc: number;
	n: number;
	title: string;
	artist: string;
	durationMs: number;
	size: number;
	onDevice: boolean;
}

export interface Album {
	id: string;
	title: string;
	artist: string;
	year?: number;
	genre?: string;
	/** Hue used for the placeholder art until real art is loaded. */
	hue: number;
	tracks: Track[];
}

export interface SyncProgress {
	active: boolean;
	current?: string;
	done: number;
	total: number;
	bytesDone: number;
	bytesTotal: number;
}

export interface Api {
	device(): Promise<DeviceStatus>;
	collection(): Promise<Album[]>;
}

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// ------------------------------------------------------------------- mock

function hash(s: string): number {
	let h = 0;
	for (const c of s) h = (h * 31 + c.charCodeAt(0)) >>> 0;
	return h;
}

const GB = 1e9;

function album(artist: string, title: string, year: number, genre: string, songs: string[], onDevice: boolean, discs = 1): Album {
	const per = Math.ceil(songs.length / discs);
	return {
		id: `${artist}--${title}`,
		title,
		artist,
		year,
		genre,
		hue: hash(artist + title) % 360,
		tracks: songs.map((t, i) => ({
			disc: Math.floor(i / per) + 1,
			n: (i % per) + 1,
			title: t,
			artist,
			durationMs: 150_000 + (hash(t) % 150_000),
			size: 6_000_000 + (hash(t) % 5_000_000),
			onDevice
		}))
	};
}

const MOCK_COLLECTION: Album[] = [
	album('100 gecs', '10,000 gecs', 2023, 'Pop', ['Dumbest Girl Alive', '757', 'Hollywood Baby', 'Frog on the Floor', 'Doritos & Fritos', 'Billy Knows Jamie', 'One Million Dollars', 'The Most Wonderful Thing', 'I Got My Tooth Removed', 'mememe'], true),
	album('100 gecs', '1000 gecs', 2019, 'Pop', ['745 sticky', 'money machine', '800 db cloud', 'I Need Help Immediately', 'stupid horse', 'xXXi_wud_nvrstøp_ÜXXx', 'ringtone', 'gecgecgec', 'hand crushed by a mallet', 'gec 2 Ü'], true),
	album('Frost Children', 'SPEED RUN', 2023, 'Electronic', ['SPEED RUN', 'FLATLINE', 'HI, HELLO', 'BAD TIMES', 'ELEVATOR', 'DUST', 'AFFIRMATIONS', 'NIGHT LIFE'], true),
	album('Jane Remover', 'Frailty', 2021, 'Alternative', ['Cage Girl', 'Pharmacy', 'Homesick', 'Royal Blue', 'Dancing with Your Eyes Closed', 'Nightcore 2010', 'Movie'], false),
	album('Porter Robinson', 'SMILE! :D plus+', 2026, 'Dance', ['Knock Yourself Out XD (Remix)', 'Cheerleader (Nitepunk Remix)', 'Russian Roulette (G Jones Remix)', 'Perfect Pinterest Garden (Remix)', 'Year of the Cup (EJEAN Remix)', 'Kitsune Maison Freestyle (Remix)', 'Cheerleader (feat. Hatsune Miku)', 'Cheerleader (Control Freak Remix)', 'Easier to Love You (Cover)', 'Is There Really No Happiness? (Remix)'], true, 2),
	album('Porter Robinson', 'Worlds 10th Anniversary Edition', 2024, 'Dance', ['Divinity', 'Sad Machine', 'Years of War', 'Flicker', 'Fresh Static Snow', 'Polygon Dust', 'Hear the Bells', 'Natural Light', 'Lionhearted', 'Sea of Voices', 'Fellow Feeling', 'Goodbye To A World'], true),
	album('Sewerslvt', 'Draining Love Story', 2020, 'Breakcore', ['Ecifircas', 'Lexapro Delirium', 'Yandere Complex', 'Cyberia lyr1', 'Down the Drain', 'Purple Hearts', 'Mr. Kill Myself'], true),
	album('torr', 'molecule', 2024, 'Shoegaze', ['electrify', 'molecule', 'safe', 'the wind', 'tame', 'blueprint', 'alter ego apocalypse', '(retcon)'], false),
	album('webcage', 'peer2peer', 2024, 'Hyperpop', ['Self-esteem', 'Twitterthreads', 'Undermyskin', 'Settings app', 'Actor', 'Stuck', 'Groupie', 'No plan', 'Fall right thru', 'Behavior', 'Head in the clouds'], false)
];

const mock: Api = {
	async device() {
		return {
			connected: true,
			name: "Abby's Zune HD",
			model: 'Zune HD',
			firmware: '4.5.114',
			capacity: 15.5 * GB,
			free: 12.6 * GB,
			usage: { music: 2.6 * GB, videos: 0, pictures: 0.2 * GB, other: 0.1 * GB }
		};
	},
	async collection() {
		return MOCK_COLLECTION;
	}
};

// ------------------------------------------------------------------ tauri

const tauri: Api = {
	async device() {
		const { invoke } = await import('@tauri-apps/api/core');
		return invoke<DeviceStatus>('device_status');
	},
	async collection() {
		const { invoke } = await import('@tauri-apps/api/core');
		return invoke<Album[]>('collection');
	}
};

export const api: Api = isTauri ? tauri : mock;
