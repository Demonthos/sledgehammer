export function work_last_created() {
    window.interpreter.Work();
}

export function last_needs_memory() {
    return window.interpreter.view.buffer.byteLength === 0;
}

export function update_last_memory(mem) {
    window.interpreter.UpdateMemory(mem);
}

let parent, len, children, node, ns, attr, op, i, name, value, element, ptr;

export class JsInterpreter {
    constructor(root, mem, _ptr_updated_ptr, _ptr_ptr, _str_ptr_ptr, _str_len_ptr) {
        this.root = root;
        this.lastNode = root;
        this.nodes = [root];
        this.parents = [];
        this.view = new DataView(mem.buffer);
        this.idSize = 1;
        this.last_start_pos;
        this.ptr_updated_ptr = _ptr_updated_ptr;
        this.ptr_ptr = _ptr_ptr;
        this.str_ptr_ptr = _str_ptr_ptr;
        this.str_len_ptr = _str_len_ptr;
        this.strings = "";
        this.strPos = 0;
        this.decoder = new TextDecoder();
        window.interpreter = this;
        this.updateDecodeIdFn();
    }

    NeedsMemory() {
        return this.view.buffer.byteLength === 0;
    }

    UpdateMemory(mem) {
        this.view = new DataView(mem.buffer);
    }

    Work() {
        if (this.view.getUint8(this.ptr_updated_ptr) === 1) {
            this.last_start_pos = this.view.getUint32(this.ptr_ptr, true);
        }
        this.u8BufPos = this.last_start_pos;
        len = this.view.getUint32(this.str_len_ptr, true);
        if (len > 0) {
            ptr = this.view.getUint32(this.str_ptr_ptr, true);
            // for small strings decoding them in javascript to avoid the overhead of native calls is faster
            if (len < 25) {
                this.strings = this.utf8Decode(ptr, len);
            }
            else {
                this.strings = this.decoder.decode(new DataView(this.view.buffer, ptr, len));
            }
            this.strPos = 0;
        }
        // this is faster than a while(true) loop
        for (; ;) {
            op = this.view.getUint8(this.u8BufPos++);
            // first bool: op & 0x20
            // second bool: op & 0x40
            // third bool: op & 0x80
            switch (op & 0x1F) {
                // append children
                case 0:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        parent = this.nodes[this.decodeId()];
                    }
                    else {
                        parent = this.lastNode;
                    }
                    len = this.decodeU32();
                    for (i = 0; i < len; i++) {
                        parent.appendChild(this.nodes[this.decodeId()]);
                    }
                    break;
                // replace with
                case 1:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        parent = this.nodes[this.decodeId()];
                    }
                    else {
                        parent = this.lastNode;
                    }
                    len = this.decodeU32();
                    if (len === 1) {
                        parent.replaceWith(this.nodes[this.decodeId()]);
                    }
                    else {
                        children = [];
                        for (i = 0; i < len; i++) {
                            children.push(this.nodes[this.decodeId()]);
                        }
                        parent.replaceWith(...children);
                    }
                    break;
                // insert after
                case 2:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        parent = this.nodes[this.decodeId()];
                    }
                    else {
                        parent = this.lastNode;
                    }
                    len = this.decodeU32();
                    if (len === 1) {
                        parent.after(this.nodes[this.decodeId()]);
                    } else {
                        children = [];
                        for (i = 0; i < len; i++) {
                            children.push(this.nodes[this.decodeId()]);
                        }
                        parent.after(...children);
                    }
                    break;
                // insert before
                case 3:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        parent = this.nodes[this.decodeId()];
                    }
                    else {
                        parent = this.lastNode;
                    }
                    len = this.decodeU32();
                    if (len === 1) {
                        parent.before(this.nodes[this.decodeId()]);
                    } else {
                        children = [];
                        for (i = 0; i < len; i++) {
                            children.push(this.nodes[this.decodeId()]);
                        }
                        parent.before(...children);
                    }
                    break;
                // remove
                case 4:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        this.nodes[this.decodeId()].remove();
                    }
                    else {
                        this.lastNode.remove();
                    }
                    break;
                // create text node
                case 5:
                    this.lastNode = document.createTextNode(this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        this.nodes[this.decodeId()] = this.lastNode;
                    }
                    break;
                // create element
                case 6:
                    name = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        this.lastNode = document.createElementNS(name, this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
                    }
                    else {
                        this.lastNode = document.createElement(name);
                    }
                    // the second bool is encoded as op & (1 << 6)
                    if (op & 0x40) {
                        this.nodes[this.decodeId()] = this.lastNode;
                    }
                    break;
                // set text
                case 7:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        this.nodes[this.decodeId()].textContent = this.strings.substring(this.strPos, this.strPos += this.decodeU16());;
                    }
                    else {
                        this.lastNode.textContent = this.strings.substring(this.strPos, this.strPos += this.decodeU16());;
                    }
                    break;
                // set attribute
                case 8:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        node = this.nodes[this.decodeId()];
                    }
                    else {
                        node = this.lastNode;
                    }
                    attr = this.view.getUint8(this.u8BufPos++);
                    switch (attr) {
                        case 254:
                            attr = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                            ns = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                            value = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                            if (ns === "style") {
                                // @ts-ignore
                                node.style[attr] = value;
                            } else if (ns != null || ns != undefined) {
                                node.setAttributeNS(ns, attr, value);
                            }
                            break;
                        case 255:
                            node.setAttribute(this.strings.substring(this.strPos, this.strPos += this.decodeU16()), this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
                            break;
                        default:
                            node.setAttribute(convertAttribute(attr), this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
                            break;
                    }
                    break;
                // remove attribute
                case 9:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        node = this.nodes[this.decodeId()];
                    }
                    else {
                        node = this.lastNode;
                    }
                    attr = this.view.getUint8(this.u8BufPos++);
                    switch (attr) {
                        case 254:
                            attr = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                            node.removeAttributeNS(this.strings.substring(this.strPos, this.strPos += this.decodeU16()), attr);
                            break;
                        case 255:
                            node.removeAttribute(this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
                            break;
                        default:
                            node.removeAttribute(convertAttribute(attr));
                            break;
                    }
                    break;
                // clone node
                case 10:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        this.lastNode = this.nodes[this.decodeId()].cloneNode(true);
                    }
                    else {
                        this.lastNode = this.lastNode.cloneNode(true);
                    }
                    // the second bool is encoded as op & (1 << 6)
                    if (op & 0x40) {
                        this.nodes[this.decodeId()] = this.lastNode;
                    }
                    break;
                // clone node children
                case 11:
                    // the first bool is encoded as op & (1 << 5)
                    if (op & 0x20) {
                        node = this.nodes[this.decodeId()].cloneNode(true).firstChild;
                    }
                    else {
                        node = this.lastNode.cloneNode(true).firstChild;
                    }
                    for (; node !== null; node = node.nextSibling) {
                        if (this.view.getUint8(this.u8BufPos++) === 1) {
                            this.nodes[this.decodeId()] = node;
                        }
                    }
                    break;
                // first child
                case 12:
                    this.lastNode = this.lastNode.firstChild;
                    break;
                // next sibling
                case 13:
                    this.lastNode = this.lastNode.nextSibling;
                    break;
                // parent
                case 14:
                    this.lastNode = this.lastNode.parentNode;
                    break;
                // store with id
                case 15:
                    this.nodes[this.decodeId()] = this.lastNode;
                    break;
                // set last node
                case 16:
                    this.lastNode = this.nodes[this.decodeId()];
                    break;
                // set id size
                case 17:
                    this.idSize = this.view.getUint8(this.u8BufPos++);
                    this.updateDecodeIdFn();
                    break;
                // stop
                case 18:
                    return;
                // create full element
                case 19:
                    this.createFullElement();
                default:
                    this.u8BufPos--;
                    return;
            }
        }
    }

    createElement() {
        element = this.view.getUint8(this.u8BufPos++);
        if (element === 255) {
            return document.createElement(this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
        }
        else {
            return document.createElement(convertElement(element));
        }
    }

    createFullElement() {
        const parent_id = this.decodeMaybeIdByteBool(),
            parent_element = this.createElement(),
            numAttributes = this.view.getUint8(this.u8BufPos++);
        for (let i = 0; i < numAttributes; i++) {
            attr = this.view.getUint8(this.u8BufPos++);
            switch (attr) {
                case 254:
                    attr = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                    ns = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                    value = this.strings.substring(this.strPos, this.strPos += this.decodeU16());
                    parent_element.setAttributeNS(ns, attr, value);
                    break;
                case 255:
                    parent_element.setAttribute(this.strings.substring(this.strPos, this.strPos += this.decodeU16()), this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
                    break;
                default:
                    parent_element.setAttribute(convertAttribute(attr), this.strings.substring(this.strPos, this.strPos += this.decodeU16()));
                    break;
            }
        }
        const numChildren = this.view.getUint8(this.u8BufPos++);
        for (let i = 0; i < numChildren; i++) {
            parent_element.appendChild(this.createFullElement());
        }
        if (parent_id !== null) {
            this.nodes[parent_id] = parent_element;
        }
        return parent_element;
    }

    // decodes and returns a node encoded with a boolean as a byte representing whether it is a new id or let last node
    decodeMaybeIdByteBool() {
        if (this.view.getUint8(this.u8BufPos++) === 0) {
            return null;
        }
        else {
            return this.decodeId();
        }
    }

    updateDecodeIdFn() {
        switch (this.idSize) {
            case 1:
                this.decodeId = function () {
                    return this.view.getUint8(this.u8BufPos++);
                };
                break;
            case 2:
                this.decodeId = function () {
                    this.u8BufPos += 2;
                    return this.view.getUint16(this.u8BufPos - 2, true);
                };
                break;
            case 4:
                this.decodeId = function () {
                    this.u8BufPos += 4;
                    return this.view.getUint32(this.u8BufPos - 4, true);
                };
                break;
        }
    }

    decodeU32() {
        this.u8BufPos += 4;
        return this.view.getUint32(this.u8BufPos - 4, true);
    }

    decodeU16() {
        this.u8BufPos += 2;
        return this.view.getUint16(this.u8BufPos - 2, true);
    }

    SetNode(id, node) {
        this.nodes[id] = node;
    }

    GetNode(id) {
        return this.nodes[id];
    }

    utf8Decode(start, byteLength) {
        let pos = start;
        const end = pos + byteLength;
        let out = "";
        let byte1;
        while (pos < end) {
            byte1 = this.view.getUint8(pos++);
            if ((byte1 & 0x80) === 0) {
                // 1 byte
                out += String.fromCharCode(byte1);
            } else if ((byte1 & 0xe0) === 0xc0) {
                // 2 bytes
                out += String.fromCharCode(((byte1 & 0x1f) << 6) | (this.view.getUint8(pos++) & 0x3f));
            } else if ((byte1 & 0xf0) === 0xe0) {
                // 3 bytes
                out += String.fromCharCode(((byte1 & 0x1f) << 12) | ((this.view.getUint8(pos++) & 0x3f) << 6) | (this.view.getUint8(pos++) & 0x3f));
            } else if ((byte1 & 0xf8) === 0xf0) {
                // 4 bytes
                let unit = ((byte1 & 0x07) << 0x12) | ((this.view.getUint8(pos++) & 0x3f) << 0x0c) | ((this.view.getUint8(pos++) & 0x3f) << 0x06) | (this.view.getUint8(pos++) & 0x3f);
                if (unit > 0xffff) {
                    unit -= 0x10000;
                    out += String.fromCharCode(((unit >>> 10) & 0x3ff) | 0xd800);
                    unit = 0xdc00 | (unit & 0x3ff);
                }
                out += String.fromCharCode(unit);
            } else {
                out += String.fromCharCode(byte1);
            }
        }

        return out;
    }
}

const els = [
    "a",
    "abbr",
    "acronym",
    "address",
    "applet",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "base",
    "bdi",
    "bdo",
    "bgsound",
    "big",
    "blink",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "center",
    "cite",
    "code",
    "col",
    "colgroup",
    "content",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "font",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "iframe",
    "image",
    "img",
    "input",
    "ins",
    "kbd",
    "keygen",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "mark",
    "marquee",
    "menu",
    "menuitem",
    "meta",
    "meter",
    "nav",
    "nobr",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "param",
    "picture",
    "plaintext",
    "portal",
    "pre",
    "progress",
    "q",
    "rb",
    "rp",
    "rt",
    "rtc",
    "ruby",
    "s",
    "samp",
    "script",
    "section",
    "select",
    "shadow",
    "slot",
    "small",
    "source",
    "spacer",
    "span",
    "strike",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "tt",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
    "xmp",
];
function convertElement(id) {
    return els[id];
}

const attrs = [
    "accept-charset",
    "accept",
    "accesskey",
    "action",
    "align",
    "allow",
    "alt",
    "aria-atomic",
    "aria-busy",
    "aria-controls",
    "aria-current",
    "aria-describedby",
    "aria-description",
    "aria-details",
    "aria-disabled",
    "aria-dropeffect",
    "aria-errormessage",
    "aria-flowto",
    "aria-grabbed",
    "aria-haspopup",
    "aria-hidden",
    "aria-invalid",
    "aria-keyshortcuts",
    "aria-label",
    "aria-labelledby",
    "aria-live",
    "aria-owns",
    "aria-relevant",
    "aria-roledescription",
    "async",
    "autocapitalize",
    "autocomplete",
    "autofocus",
    "autoplay",
    "background",
    "bgcolor",
    "border",
    "buffered",
    "capture",
    "challenge",
    "charset",
    "checked",
    "cite",
    "class",
    "code",
    "codebase",
    "color",
    "cols",
    "colspan",
    "content",
    "contenteditable",
    "contextmenu",
    "controls",
    "coords",
    "crossorigin",
    "csp",
    "data",
    "datetime",
    "decoding",
    "default",
    "defer",
    "dir",
    "dirname",
    "disabled",
    "download",
    "draggable",
    "enctype",
    "enterkeyhint",
    "for",
    "form",
    "formaction",
    "formenctype",
    "formmethod",
    "formnovalidate",
    "formtarget",
    "headers",
    "height",
    "hidden",
    "high",
    "href",
    "hreflang",
    "http-equiv",
    "icon",
    "id",
    "importance",
    "inputmode",
    "integrity",
    "intrinsicsize",
    "ismap",
    "itemprop",
    "keytype",
    "kind",
    "label",
    "lang",
    "language",
    "list",
    "loading",
    "loop",
    "low",
    "manifest",
    "max",
    "maxlength",
    "media",
    "method",
    "min",
    "minlength",
    "multiple",
    "muted",
    "name",
    "novalidate",
    "open",
    "optimum",
    "pattern",
    "ping",
    "placeholder",
    "poster",
    "preload",
    "radiogroup",
    "readonly",
    "referrerpolicy",
    "rel",
    "required",
    "reversed",
    "role",
    "rows",
    "rowspan",
    "sandbox",
    "scope",
    "scoped",
    "selected",
    "shape",
    "size",
    "sizes",
    "slot",
    "span",
    "spellcheck",
    "src",
    "srcdoc",
    "srclang",
    "srcset",
    "start",
    "step",
    "style",
    "summary",
    "tabindex",
    "target",
    "title",
    "translate",
    "type",
    "usemap",
    "value",
    "width",
    "wrap",
];
function convertAttribute(id) {
    return attrs[id];
}

const events = [
    "abort",
    "activate",
    "addstream",
    "addtrack",
    "afterprint",
    "afterscriptexecute",
    "animationcancel",
    "animationend",
    "animationiteration",
    "animationstart",
    "appinstalled",
    "audioend",
    "audioprocess",
    "audiostart",
    "auxclick",
    "beforeinput",
    "beforeprint",
    "beforescriptexecute",
    "beforeunload",
    "beginEvent",
    "blocked",
    "blur",
    "boundary",
    "bufferedamountlow",
    "cancel",
    "canplay",
    "canplaythrough",
    "change",
    "click",
    "close",
    "closing",
    "complete",
    "compositionend",
    "compositionstart",
    "compositionupdate",
    "connect",
    "connectionstatechange",
    "contentdelete",
    "contextmenu",
    "copy",
    "cuechange",
    "cut",
    "datachannel",
    "dblclick",
    "devicechange",
    "devicemotion",
    "deviceorientation",
    "DOMActivate",
    "DOMContentLoaded",
    "DOMMouseScroll",
    "drag",
    "dragend",
    "dragenter",
    "dragleave",
    "dragover",
    "dragstart",
    "drop",
    "durationchange",
    "emptied",
    "end",
    "ended",
    "endEvent",
    "enterpictureinpicture",
    "error",
    "focus",
    "focusin",
    "focusout",
    "formdata",
    "fullscreenchange",
    "fullscreenerror",
    "gamepadconnected",
    "gamepaddisconnected",
    "gatheringstatechange",
    "gesturechange",
    "gestureend",
    "gesturestart",
    "gotpointercapture",
    "hashchange",
    "icecandidate",
    "icecandidateerror",
    "iceconnectionstatechange",
    "icegatheringstatechange",
    "input",
    "inputsourceschange",
    "install",
    "invalid",
    "keydown",
    "keypress",
    "keyup",
    "languagechange",
    "leavepictureinpicture",
    "load",
    "loadeddata",
    "loadedmetadata",
    "loadend",
    "loadstart",
    "lostpointercapture",
    "mark",
    "merchantvalidation",
    "message",
    "messageerror",
    "mousedown",
    "mouseenter",
    "mouseleave",
    "mousemove",
    "mouseout",
    "mouseover",
    "mouseup",
    "mousewheel",
    "msContentZoom",
    "u8BufestureChange",
    "u8BufestureEnd",
    "u8BufestureHold",
    "u8BufestureStart",
    "u8BufestureTap",
    "MSInertiaStart",
    "MSManipulationStateChanged",
    "mute",
    "negotiationneeded",
    "nomatch",
    "notificationclick",
    "offline",
    "online",
    "open",
    "orientationchange",
    "pagehide",
    "pageshow",
    "paste",
    "pause",
    "payerdetailchange",
    "paymentmethodchange",
    "play",
    "playing",
    "pointercancel",
    "pointerdown",
    "pointerenter",
    "pointerleave",
    "pointerlockchange",
    "pointerlockerror",
    "pointermove",
    "pointerout",
    "pointerover",
    "pointerup",
    "popstate",
    "progress",
    "push",
    "pushsubscriptionchange",
    "ratechange",
    "readystatechange",
    "rejectionhandled",
    "removestream",
    "removetrack",
    "removeTrack",
    "repeatEvent",
    "reset",
    "resize",
    "resourcetimingbufferfull",
    "result",
    "resume",
    "scroll",
    "search",
    "seeked",
    "seeking",
    "select",
    "selectedcandidatepairchange",
    "selectend",
    "selectionchange",
    "selectstart",
    "shippingaddresschange",
    "shippingoptionchange",
    "show",
    "signalingstatechange",
    "slotchange",
    "soundend",
    "soundstart",
    "speechend",
    "speechstart",
    "squeeze",
    "squeezeend",
    "squeezestart",
    "stalled",
    "start",
    "statechange",
    "storage",
    "submit",
    "success",
    "suspend",
    "timeout",
    "timeupdate",
    "toggle",
    "tonechange",
    "touchcancel",
    "touchend",
    "touchmove",
    "touchstart",
    "track",
    "transitioncancel",
    "transitionend",
    "transitionrun",
    "transitionstart",
    "unhandledrejection",
    "unload",
    "unmute",
    "upgradeneeded",
    "versionchange",
    "visibilitychange",
    "voiceschanged",
    "volumechange",
    "vrdisplayactivate",
    "vrdisplayblur",
    "vrdisplayconnect",
    "vrdisplaydeactivate",
    "vrdisplaydisconnect",
    "vrdisplayfocus",
    "vrdisplaypointerrestricted",
    "vrdisplaypointerunrestricted",
    "vrdisplaypresentchange",
    "waiting",
    "webglcontextcreationerror",
    "webglcontextlost",
    "webglcontextrestored",
    "webkitmouseforcechanged",
    "webkitmouseforcedown",
    "webkitmouseforceup",
    "webkitmouseforcewillbegin",
    "wheel",
];
function convertEvent(id) {
    return events[id];
}