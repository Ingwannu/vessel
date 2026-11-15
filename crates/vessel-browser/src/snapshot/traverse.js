function (config) {
  "use strict";
  // Independently authored from the input/output requirements. No reference code.
  const stateKey = "__vessel_owned_snapshot_v1";
  const bindings = new Map();
  globalThis[stateKey] = bindings; // Invalidates even when this traversal later throws.
  const limits = config.limits;
  const nodes = [];
  const encoder = new TextEncoder();
  let next = BigInt(config.firstRef);
  let retained = 0;
  let remaining = limits.textBytes;
  let visited = 0;
  let frames = 0;
  let truncated = false;
  const interactive = new Set(["button", "link", "textbox", "searchbox", "checkbox", "radio",
    "combobox", "listbox", "option", "slider", "spinbutton", "switch", "tab", "menuitem", "treeitem"]);
  const roles = new Set(config.roles);
  function takeWork() {
    if (visited >= limits.nodes) { truncated = true; return false; }
    visited += 1;
    return true;
  }
  function measured(value, maximum) {
    const text = String(value ?? "");
    if (text.length > maximum) return null;
    const bytes = encoder.encode(text).length;
    return bytes <= maximum ? { text, bytes } : null;
  }
  function text(value) {
    const raw = String(value ?? "");
    let end = Math.min(raw.length, remaining);
    if (end < raw.length && end > 0 && /[\uD800-\uDBFF]/.test(raw[end - 1])) end -= 1;
    let prefix = raw.slice(0, end);
    while (encoder.encode(prefix).length > remaining) {
      end = Math.floor(end / 2);
      if (end > 0 && /[\uD800-\uDBFF]/.test(raw[end - 1])) end -= 1;
      prefix = raw.slice(0, end);
    }
    const bytes = encoder.encode(prefix).length;
    remaining -= bytes;
    if (prefix.length !== raw.length) truncated = true;
    return prefix;
  }
  function attr(element, name) { return element.getAttribute(name) ?? ""; }
  function hidden(element) {
    if (element.hidden || attr(element, "aria-hidden") === "true") return true;
    const style = element.ownerDocument.defaultView.getComputedStyle(element);
    return style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse";
  }
  function descendantsText(root) {
    // Share the traversal's work budget: label scans cannot multiply an unbounded walk.
    const iterator = root.ownerDocument.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let result = "";
    let item;
    while ((item = iterator.nextNode()) !== null) {
      if (!takeWork()) break;
      const parent = item.parentElement;
      if (parent && !hidden(parent) && !["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE"].includes(parent.tagName)) {
        const available = remaining - encoder.encode(result).length;
        const part = measured(item.nodeValue, Math.max(0, available));
        if (!part) { truncated = true; break; }
        result += part.text;
      }
    }
    return result.replace(/\s+/gu, " ").trim();
  }
  function label(element, role) {
    const labelled = attr(element, "aria-labelledby");
    if (labelled && measured(labelled, remaining)) {
      let name = "";
      const matcher = /\S+/gu;
      let match;
      while ((match = matcher.exec(labelled)) !== null && visited < limits.nodes) {
        const other = element.ownerDocument.getElementById(match[0]);
        if (other) name += (name ? " " : "") + descendantsText(other);
        if (!measured(name, remaining)) { truncated = true; return ""; }
      }
      if (name) return name;
    }
    const aria = attr(element, "aria-label");
    if (aria) return aria;
    let name = "";
    const labels = element.labels;
    if (labels) {
      for (let index = 0; index < labels.length && visited < limits.nodes; index += 1) {
        name += (name ? " " : "") + descendantsText(labels[index]);
        if (!measured(name, remaining)) { truncated = true; return ""; }
      }
    }
    if (name) return name;
    if (role === "img") return attr(element, "alt") || attr(element, "title");
    if (["textbox", "searchbox", "combobox", "spinbutton", "slider"].includes(role))
      return attr(element, "placeholder") || attr(element, "title");
    if (element.tagName === "INPUT" && role === "button") return element.value;
    if (["button", "link", "heading", "option", "menuitem", "tab", "treeitem"].includes(role))
      return descendantsText(element) || attr(element, "title");
    return attr(element, "title");
  }
  function roleOf(element) {
    const supplied = attr(element, "role");
    const explicit = measured(supplied, remaining) ? supplied.trim().split(/\s+/u, 1)[0] : "";
    if (roles.has(explicit)) return explicit;
    const tag = element.tagName;
    if (/^H[1-6]$/u.test(tag)) return "heading";
    if (element.isContentEditable) return "textbox";
    if (tag === "INPUT") {
      const type = element.type;
      if (["button", "submit", "reset", "image"].includes(type)) return "button";
      return ({ checkbox:"checkbox", radio:"radio", range:"slider", number:"spinbutton",
        search:"searchbox", hidden:null })[type] ?? (type === "hidden" ? null : "textbox");
    }
    if (tag === "A" && !element.hasAttribute("href")) return null;
    if (tag === "SELECT") return element.multiple || element.size > 1 ? "listbox" : "combobox";
    return ({ BUTTON:"button", A:"link", TEXTAREA:"textbox", OPTION:"option", IMG:"img",
      NAV:"navigation", MAIN:"main", HEADER:"banner", FOOTER:"contentinfo", ASIDE:"complementary",
      FORM:"form", ARTICLE:"article", UL:"list", OL:"list", LI:"listitem", TABLE:"table",
      TR:"row", TD:"cell", TH:"columnheader", DIALOG:"dialog", PROGRESS:"progressbar",
      FIELDSET:"group", IFRAME:"iframe", HR:"separator" })[tag] ?? null;
  }
  function stateOf(element, role) {
    const tri = attr(element, "aria-checked");
    let checked = ["true", "false", "mixed"].includes(tri) ? tri : null;
    if (!checked && ["checkbox", "radio"].includes(role)) checked = element.indeterminate ? "mixed" : String(Boolean(element.checked));
    const booleanAttribute = name => ["true", "false"].includes(attr(element, name)) ? attr(element, name) === "true" : null;
    const heading = /^H([1-6])$/u.exec(element.tagName);
    return { checked, expanded:booleanAttribute("aria-expanded"),
      selected:role === "option" ? Boolean(element.selected) : booleanAttribute("aria-selected"),
      disabled:element.matches(":disabled") || attr(element, "aria-disabled") === "true",
      readonly:Boolean(element.readOnly) || attr(element, "aria-readonly") === "true",
      level:role === "heading" && heading ? Number(heading[1]) : null };
  }
  function editableValue(element) {
    if (!element.isContentEditable) return typeof element.value === "string" ? element.value : "";
    const iterator = element.ownerDocument.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    let result = "";
    let used = 0;
    let item;
    while ((item = iterator.nextNode()) !== null) {
      if (!takeWork()) return null;
      const part = measured(item.nodeValue, remaining - used);
      if (!part) { truncated = true; return null; }
      result += part.text;
      used += part.bytes;
    }
    return result;
  }
  function bind(element, role, name) {
    const empty = { tag:"", id:"", input_type:"", name:"", value:"" };
    if (!interactive.has(role) || retained >= limits.references) {
      if (interactive.has(role)) truncated = true;
      return { reference:null, fingerprint:empty };
    }
    const value = editableValue(element);
    if (value === null) return { reference:null, fingerprint:empty };
    const fingerprint = { tag:element.tagName.toLowerCase(), id:element.id,
      input_type:attr(element, "type"), name:attr(element, "name"), value };
    let required = 0;
    for (const field of Object.values(fingerprint)) {
      const size = measured(field, remaining - required);
      if (!size) { truncated = true; return { reference:null, fingerprint:empty }; }
      required += size.bytes;
    }
    remaining -= required;
    const reference = "@e" + next.toString();
    next += 1n;
    retained += 1;
    bindings.set(reference, { element, fingerprint, role, name });
    return { reference, fingerprint };
  }
  function children(root, depth, domDepth) {
    let child = root.firstChild;
    while (child && visited < limits.nodes && remaining > 0) {
      visit(child, depth, domDepth);
      child = child.nextSibling;
    }
    if (child) truncated = true;
  }
  function visit(node, depth, domDepth) {
    if (!takeWork()) return;
    if (domDepth > limits.depth || depth > limits.depth) { truncated = true; return; }
    if (node.nodeType === Node.TEXT_NODE) {
      const value = text(node.nodeValue).replace(/\s+/gu, " ").trim();
      if (value) nodes.push({ kind:"text", depth, text:value });
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;
    const element = node;
    if (["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE"].includes(element.tagName) || hidden(element)) return;
    const role = roleOf(element);
    let nextDepth = depth;
    if (role) {
      const name = text(label(element, role));
      nodes.push({ kind:"element", depth, role, name, ...bind(element, role, name), state:stateOf(element, role) });
      nextDepth += 1;
    }
    if (element.tagName === "IFRAME") {
      frames += 1;
      // No universal access. A cross-origin frame stays an iframe descriptor.
      const doc = element.contentDocument;
      if (doc?.body) children(doc.body, nextDepth, domDepth + 1);
      return;
    }
    if (interactive.has(role) || ["heading", "img", "progressbar", "separator"].includes(role)) return;
    if (element.shadowRoot) children(element.shadowRoot, nextDepth, domDepth + 1);
    else children(element, nextDepth, domDepth + 1);
  }
  if (!document.body) throw new Error("snapshot_document_unavailable");
  if (!takeWork()) throw new Error("snapshot_budget_unavailable");
  nodes.push({ kind:"element", depth:0, role:"document", name:text(document.title),
    reference:null, fingerprint:{ tag:"", id:"", input_type:"", name:"", value:"" },
    state:{ checked:null, expanded:null, selected:null, disabled:false, readonly:false, level:null } });
  children(document.body, 1, 1);
  return { nodes, visited, frames, truncated };
}
