async function(config) {
    try {
        const callable = (0, eval)("(" + config.source + "\n)");
        if (typeof callable !== "function") return {kind: "not_function"};
        const observed = await callable();
        const json = JSON.stringify(observed) ?? "null";
        const encoder = new TextEncoder();
        if (encoder.encode(json).length <= config.limit) {
            return {kind: "value", text: json, completeness: "whole"};
        }
        const chunks = [];
        let previewBytes = 0;
        let resultBytes = 2;
        for (const character of json) {
            const plainBytes = encoder.encode(character).length;
            const escaped = JSON.stringify(character).slice(1, -1);
            const escapedBytes = encoder.encode(escaped).length;
            if (previewBytes + plainBytes > config.preview ||
                resultBytes + escapedBytes > config.limit) break;
            chunks.push(escaped);
            previewBytes += plainBytes;
            resultBytes += escapedBytes;
        }
        return {kind: "value", text: '"' + chunks.join("") + '"', completeness: "truncated"};
    } catch {
        return {kind: "exception"};
    }
}
