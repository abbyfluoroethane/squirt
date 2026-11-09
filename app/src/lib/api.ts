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
	id?: number;
	codec?: string;
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
	/** Hue used for placeholder art when there is no cover. */
	hue: number;
	/** URL of the cover art thumbnail. */
	art?: string | null;
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

export type PlayStatus = 'stopped' | 'playing' | 'paused';

export interface PlayerState {
	status: PlayStatus;
	trackId: number | null;
	positionMs: number;
	durationMs: number;
	volume: number;
	queue: number[];
}

export interface PlayerApi {
	state(): Promise<PlayerState>;
	play(ids: number[], start: number): Promise<PlayerState>;
	toggle(): Promise<PlayerState>;
	next(): Promise<PlayerState>;
	prev(): Promise<PlayerState>;
	seek(ms: number): Promise<PlayerState>;
	volume(v: number): Promise<PlayerState>;
}

export interface Api {
	device(): Promise<DeviceStatus>;
	collection(): Promise<Album[]>;
	player: PlayerApi;
}

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Which backend the UI is talking to. */
export type Backend = 'tauri' | 'dev-api' | 'mock';

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
			id: hash(artist + title + t),
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

/** A pretend player for the mock backend: keeps time with the wall clock. */
function mockPlayer(tracks: () => Track[]): PlayerApi {
	let st: PlayerState = { status: 'stopped', trackId: null, positionMs: 0, durationMs: 0, volume: 0.7, queue: [] };
	let startedAt = 0;
	let basePos = 0;
	const dur = (id: number | null) => tracks().find((t) => t.id === id)?.durationMs ?? 0;
	const now = (): PlayerState => {
		if (st.status === 'playing') {
			st.positionMs = basePos + (Date.now() - startedAt);
			if (st.positionMs >= st.durationMs) {
				const i = st.queue.indexOf(st.trackId!);
				if (i + 1 < st.queue.length) go(i + 1, 0);
				else st = { ...st, status: 'stopped', trackId: null, positionMs: 0, durationMs: 0 };
			}
		}
		return { ...st };
	};
	const go = (i: number, pos: number) => {
		st.trackId = st.queue[i];
		st.durationMs = dur(st.trackId);
		st.positionMs = basePos = pos;
		startedAt = Date.now();
		st.status = 'playing';
	};
	return {
		async state() {
			return now();
		},
		async play(ids, start) {
			st.queue = ids;
			go(start, 0);
			return now();
		},
		async toggle() {
			now();
			if (st.status === 'playing') {
				st.status = 'paused';
				basePos = st.positionMs;
			} else if (st.trackId !== null) {
				st.status = 'playing';
				startedAt = Date.now();
			}
			return now();
		},
		async next() {
			const i = st.queue.indexOf(st.trackId!);
			if (i >= 0 && i + 1 < st.queue.length) go(i + 1, 0);
			return now();
		},
		async prev() {
			const i = st.queue.indexOf(st.trackId!);
			if (i >= 0) go(st.positionMs < 3000 && i > 0 ? i - 1 : i, 0);
			return now();
		},
		async seek(ms) {
			if (st.trackId !== null) {
				const paused = st.status === 'paused';
				basePos = ms;
				startedAt = Date.now();
				st.positionMs = ms;
				if (paused) st.status = 'paused';
			}
			return now();
		},
		async volume(v) {
			st.volume = v;
			return now();
		}
	};
}

const mock: Api = {
	player: mockPlayer(() => MOCK_COLLECTION.flatMap((a) => a.tracks)),
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

const tauriPlayer: PlayerApi = {
	state: () => call('player_state'),
	play: (ids, start) => call('player_play', { ids, start }),
	toggle: () => call('player_toggle'),
	next: () => call('player_next'),
	prev: () => call('player_prev'),
	seek: (ms) => call('player_seek', { ms }),
	volume: (v) => call('player_volume', { v })
};

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke<T>(cmd, args);
}

const tauri: Api = {
	player: tauriPlayer,
	async device() {
		const { invoke } = await import('@tauri-apps/api/core');
		return invoke<DeviceStatus>('device_status');
	},
	async collection() {
		const { invoke } = await import('@tauri-apps/api/core');
		return invoke<Album[]>('collection');
	}
};

// ---------------------------------------------------------------- dev api

// `squirt serve` (proxied by Vite at /api) exposes the real library to a
// plain browser during development.
const post = async (path: string): Promise<PlayerState> => (await fetch(`/api/player/${path}`, { method: 'POST' })).json();

const devApi: Api = {
	player: {
		state: async () => (await fetch('/api/player/state')).json(),
		play: (ids, start) => post(`play?ids=${ids.join(',')}&start=${start}`),
		toggle: () => post('toggle'),
		next: () => post('next'),
		prev: () => post('prev'),
		seek: (ms) => post(`seek?ms=${Math.round(ms)}`),
		volume: (v) => post(`volume?v=${v.toFixed(3)}`)
	},
	async device() {
		return (await fetch('/api/device')).json();
	},
	async collection() {
		return (await fetch('/api/collection')).json();
	}
};

async function devApiRunning(): Promise<boolean> {
	try {
		const r = await fetch('/api/health', { signal: AbortSignal.timeout(800) });
		return r.ok;
	} catch {
		return false;
	}
}

/** Picks the backend: Tauri in the app, else the dev API if running, else mock data. */
export async function connect(): Promise<{ api: Api; backend: Backend }> {
	if (isTauri) return { api: tauri, backend: 'tauri' };
	if (await devApiRunning()) return { api: devApi, backend: 'dev-api' };
	return { api: mock, backend: 'mock' };
}
