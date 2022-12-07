// https://marcoghiani.com/blog/write-a-javascript-fetch-wrapper-in-less-than-1kb

interface RequestInformation {
    headers?: HeadersInit;
    method: string;
    body: object | string; // changed from string
    cache?: RequestCache;
    credentials?: RequestCredentials;
    integrity?: string;
    keepalive?: boolean;
    mode?: RequestMode;
    redirect?: RequestRedirect;
    referrer?: string;
    referrerPolicy?: ReferrerPolicy;
    signal?: AbortSignal;
    window?: null;
}

export async function request(
    url: RequestInfo | URL,
    options?: RequestInformation
): Promise<{ data: any; ok: boolean }> {
    const { headers, method = "GET", body, ...extraOpts } = options || {};

    const reqOptions: RequestInit = {
        method,
        headers: {
            "Content-type": "application/json",
            ...headers,
        },
        ...extraOpts,
    };

    if (body) {
        reqOptions.body = typeof body === "object" ? JSON.stringify(body) : body;
    }

    const res = await fetch(url, reqOptions);
    const data = await parseJSON(res);

    if (res.status == 200) {
        return { data, ok: true };
    } else if (res.status == 401) {
        console.log("Refreshing");
        const isRefreshed = await tryRefreshToken();
        if (isRefreshed) {
            const res = await fetch(url, reqOptions);
            const data = await parseJSON(res);
            if (res.status == 200) return { data, ok: true };
        }
        // ask for login
        console.log("Expired access token");
    } else {
        // bad request, ...
        //throw new Error(data.error_info);
    }
    return { data, ok: false };
}

export async function tryRefreshToken() {
    const res = await fetch("/api/auth/refresh", { method: "POST" });
    return res.ok;
}

async function parseJSON(res: Response) {
    try {
        const json = await res.json();
        return json;
    } catch (e) {
        //console.log(e)
        return {};
    }
}

export const checkAvailability = () => {
    console.log("Checking availability");
    return fetch(`/api/health`)
        .then((res) => {
            return res.ok;
        })
        .catch((e) => {
            console.log(e);
            return false;
        });
};
