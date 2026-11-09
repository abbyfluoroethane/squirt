// Keeps the Social tab's data between visits (and across launches, for the
// profile), so it opens instantly and refreshes in the background.
import type { LastFmProfile, SocialStatus } from '$lib/api';

const KEY = 'squirt.social.profile';

function load(): LastFmProfile | null {
	try {
		const raw = localStorage.getItem(KEY);
		return raw ? (JSON.parse(raw) as LastFmProfile) : null;
	} catch {
		return null;
	}
}

class SocialCache {
	status = $state<SocialStatus | null>(null);
	profile = $state<LastFmProfile | null>(typeof localStorage === 'undefined' ? null : load());

	setProfile(p: LastFmProfile | null) {
		this.profile = p;
		try {
			if (p) localStorage.setItem(KEY, JSON.stringify(p));
			else localStorage.removeItem(KEY);
		} catch {
			// Not saved; still cached for this session.
		}
	}
}

export const socialCache = new SocialCache();
