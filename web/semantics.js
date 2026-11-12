// Transparent platform input and accessibility projection. Rust supplies every
// visible rectangle, string, state and default action; DOM measurement is unused.
export function droppedFileMetadata(files) {
  const list = Array.from(files || []),
    bytes = (value) => new TextEncoder().encode(value).length;
  if (!list.length || list.length > 16) throw new Error('Invalid dropped-file count');
  return list.map((file) => {
    const name = file.name,
      mime = file.type || null,
      size = file.size;
    if (
      typeof name !== 'string' ||
      !name ||
      bytes(name) > 1024 ||
      ['.', '..'].includes(name) ||
      /[/\\\x00-\x1f\x7f-\x9f]/.test(name) ||
      (mime !== null &&
        (typeof mime !== 'string' || mime.length > 256 || /[^\x20-\x7e]/.test(mime))) ||
      !Number.isSafeInteger(size) ||
      size < 0
    )
      throw new Error('Invalid dropped-file metadata');
    return { name, mime, size };
  });
}
export class SemanticProjection {
  constructor(root, app, schedule, pump, reveal) {
    Object.assign(this, { root, app, schedule, pump, reveal });
    this.elements = new Map();
    this.composing = new Set();
    this.lastModal = null;
    this.restoreFocus = null;
    this.onSelection = () => {
      const el = document.activeElement;
      if (root.contains(el) && this.editable(el)) this.selection(el);
    };
    document.addEventListener('selectionchange', this.onSelection);
  }
  editable(el) {
    return el?.tagName === 'INPUT' || el?.tagName === 'TEXTAREA';
  }
  drop(el, native) {
    native.preventDefault();
    native.stopPropagation();
    let extra;
    try {
      extra = {
        files: droppedFileMetadata(native.dataTransfer?.files),
        x: native.clientX,
        y: native.clientY,
      };
    } catch {
      const data = {
        resource_error: { source: 'service', code: 'invalid_drop_metadata', status: null },
      };
      if (el) this.event(el, 'resource_error', data, native);
      else this.hostEvent('resource_error', data, native);
      return;
    }
    if (el) this.event(el, 'file_drop', extra, native);
    else this.hostEvent('file_drop', extra, native);
  }
  hostEvent(kind, extra = {}, native) {
    if (!this.app.host_event) return false;
    const result = JSON.parse(this.app.host_event(JSON.stringify({ kind, ...extra })));
    if (result.default_prevented && native?.cancelable) native.preventDefault();
    this.pump();
    this.schedule();
    return !!result.default_prevented;
  }
  event(el, kind, extra = {}, native) {
    if (!this.app.dispatch) return false;
    const result = JSON.parse(
      this.app.dispatch(Number(el.dataset.node), JSON.stringify({ kind, ...extra })),
    );
    if (result.default_prevented && native?.cancelable) native.preventDefault();
    if (result.pointer_capture && native?.pointerId !== undefined)
      (this.elements.get(result.pointer_capture) || el).setPointerCapture(native.pointerId);
    if (
      result.pointer_released &&
      native?.pointerId !== undefined &&
      el.hasPointerCapture(native.pointerId)
    )
      el.releasePointerCapture(native.pointerId);
    if (
      result.handled &&
      result.default_prevented &&
      kind.startsWith('pointer_') &&
      this.editable(el)
    ) {
      if (kind === 'pointer_down') el.focus({ preventScroll: true });
      this.syncEditor(el);
    }
    this.pump();
    this.schedule();
    return !!result.default_prevented;
  }
  selection(el) {
    if (el.selectionStart === null || el.selectionStart === undefined) {
      this.app.edit_sync?.(
        Number(el.dataset.node),
        el.value,
        el.value.length,
        el.value.length,
        this.composing.has(el),
      );
      this.schedule();
      return;
    }
    const a = el.selectionDirection === 'backward' ? el.selectionEnd : el.selectionStart,
      b = el.selectionDirection === 'backward' ? el.selectionStart : el.selectionEnd;
    this.app.edit_sync?.(Number(el.dataset.node), el.value, a, b, this.composing.has(el));
    this.schedule();
  }
  syncEditor(el) {
    if (!this.editable(el) || this.composing.has(el) || !this.app.edit_selection) return;
    const id = Number(el.dataset.node),
      selection = JSON.parse(this.app.edit_selection(id)),
      value = this.app.edit_value?.(id);
    if (typeof value === 'string' && el.value !== value) el.value = value;
    if (selection && el.type !== 'email') {
      const [a, b] = selection;
      el.setSelectionRange(Math.min(a, b), Math.max(a, b), a > b ? 'backward' : 'forward');
    }
  }
  create(n) {
    const c = n.control || {},
      tag =
        n.role === 'textbox' || c.editable
          ? c.multiline
            ? 'textarea'
            : 'input'
          : n.role === 'link'
            ? 'a'
            : n.focusable
              ? 'button'
              : 'span';
    const el = document.createElement(tag);
    el.id = `rustx-${n.id}`;
    el.dataset.node = n.id;
    el.className = tag === 'span' ? 'text' : 'control';
    if (tag === 'button') el.type = 'button';
    el.addEventListener('focus', () => {
      this.app.focus(n.id);
      this.reveal(n.id);
      this.schedule();
    });
    el.addEventListener('blur', () => {
      this.app.blur?.(n.id);
      this.event(el, 'blur');
    });
    el.addEventListener('dragover', (e) => e.preventDefault());
    el.addEventListener('drop', (e) => this.drop(el, e));
    if (tag === 'input' || tag === 'textarea') {
      el.autocomplete = c.input_type === 'password' ? 'current-password' : 'off';
      el.spellcheck = false;
      el.addEventListener('beforeinput', (e) =>
        this.event(
          el,
          'before_input',
          { value: e.data || '', input_type: e.inputType || '', composing: e.isComposing },
          e,
        ),
      );
      el.addEventListener('input', (e) => {
        if (this.app.edit_sync) this.selection(el);
        else this.app.input(n.id, el.value, e.isComposing);
        this.pump();
        this.schedule();
      });
      el.addEventListener('select', () => this.selection(el));
      el.addEventListener('compositionstart', (e) => {
        this.composing.add(el);
        this.event(el, 'composition_start', { value: e.data || '' });
      });
      el.addEventListener('compositionupdate', (e) =>
        this.event(el, 'composition_update', { value: e.data || '' }),
      );
      el.addEventListener('compositionend', (e) => {
        this.composing.delete(el);
        this.selection(el);
        this.event(el, 'composition_end', { value: e.data || '' });
      });
      for (const kind of ['copy', 'cut', 'paste'])
        el.addEventListener(kind, (e) => this.event(el, kind, {}, e));
    } else if (n.focusable) {
      el.addEventListener('click', (e) => {
        if (tag === 'a') {
          this.event(
            el,
            'click',
            { ctrl: e.ctrlKey, meta: e.metaKey, shift: e.shiftKey, alt: e.altKey },
            e,
          );
          return;
        }
        this.app.activate(n.id);
        this.pump();
        this.schedule();
      });
    }
    el.addEventListener('keydown', (e) => {
      // Browser/OS shortcuts keep their native behavior unless Rust handles them.
      const prevented = this.event(
        el,
        'key_down',
        { key: e.key, shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey },
        e,
      );
      let handled = false;
      if (!prevented && e.key !== 'Tab' && this.app.key_event)
        handled = this.app.key_event(n.id, e.key, e.shiftKey, e.ctrlKey, e.altKey, e.metaKey);
      if (handled) e.preventDefault();
      // A handled key changes Rust before the next visual frame. Update the
      // platform bridge now so queued selection events and subsequent native
      // typing cannot send its previous value/caret back over that edit.
      if (prevented || handled) this.syncEditor(el);
      this.schedule();
    });
    el.addEventListener('keyup', (e) =>
      this.event(
        el,
        'key_up',
        { key: e.key, shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey },
        e,
      ),
    );
    for (const [native, kind] of [
      ['pointerdown', 'pointer_down'],
      ['pointermove', 'pointer_move'],
      ['pointerup', 'pointer_up'],
      ['pointercancel', 'pointer_cancel'],
      ['pointerenter', 'pointer_enter'],
      ['pointerleave', 'pointer_leave'],
    ]) {
      el.addEventListener(native, (e) =>
        this.event(
          el,
          kind,
          {
            pointer_id: e.pointerId,
            x: e.clientX,
            y: e.clientY,
            button: e.button,
            buttons: e.buttons,
            pointer_type: e.pointerType,
            shift: e.shiftKey,
            ctrl: e.ctrlKey,
            meta: e.metaKey,
            alt: e.altKey,
          },
          e,
        ),
      );
    }
    if (n.role === 'img' && n.focusable) {
      let pointer;
      el.addEventListener('pointerdown', (e) => {
        if (e.defaultPrevented) return;
        pointer = [e.clientX, e.clientY, false];
        el.setPointerCapture(e.pointerId);
        el.focus({ preventScroll: true });
        e.preventDefault();
      });
      el.addEventListener('pointermove', (e) => {
        if (e.defaultPrevented) return;
        if (!pointer) return;
        const dx = e.clientX - pointer[0],
          dy = e.clientY - pointer[1];
        if (dx || dy) {
          this.app.canvas_pan(n.id, dx, dy);
          pointer = [e.clientX, e.clientY, true];
          this.schedule();
        }
      });
      el.addEventListener('pointerup', (e) => {
        if (e.defaultPrevented) {
          pointer = null;
          return;
        }
        if (pointer && !pointer[2]) this.app.canvas_pick(n.id, e.clientX, e.clientY);
        pointer = null;
        this.schedule();
      });
      el.addEventListener('pointercancel', () => {
        pointer = null;
      });
    }
    this.root.append(el);
    this.elements.set(n.id, el);
    return el;
  }
  sync(semantic, inspection) {
    const boxes = new Map(inspection.nodes.map((n) => [n.node, n])),
      seen = new Set(),
      ownership = new Map(),
      flat = [];
    const walk = (n, owner = null) => {
      n.owner = owner;
      flat.push(n);
      for (const child of n.children || []) walk(child, n.role === 'group' ? owner : n.id);
    };
    walk(semantic);
    const relation = (value) =>
      value
        ?.split(/\s+/)
        .filter(Boolean)
        .map((id) => {
          const target = flat.find((n) => n.app_id === id);
          return target ? `rustx-${target.id}` : id;
        })
        .join(' ');
    let modal;
    this.flat = flat;
    for (const n of flat) {
      if (n.control?.hidden) continue;
      if (n.role === 'group' && !n.label && !n.control?.modal) continue;
      const g = boxes.get(n.id);
      if (!g) continue;
      seen.add(n.id);
      const c = n.control || {};
      let el = this.elements.get(n.id) || this.create(n);
      el.dataset.testid = n.app_id || '';
      el.dataset.scrollNode = g.scroll_parent || '';
      if (n.owner) {
        const owned = ownership.get(n.owner) || [];
        owned.push(el.id);
        ownership.set(n.owner, owned);
      }
      const attr = (key, value) => {
        if (value === null || value === undefined || value === '') el.removeAttribute(key);
        else el.setAttribute(key, String(value));
      };
      attr('role', n.role === 'text' ? null : n.role);
      attr('aria-label', n.label || null);
      attr('aria-disabled', c.disabled || null);
      attr('aria-readonly', c.read_only || null);
      for (const key of ['checked', 'selected', 'expanded', 'modal']) attr(`aria-${key}`, c[key]);
      attr('aria-describedby', relation(c.described_by));
      attr('aria-labelledby', relation(c.labelled_by));
      attr('aria-controls', relation(c.controls));
      attr('aria-valuemin', c.min);
      attr('aria-valuemax', c.max);
      attr('aria-valuenow', n.role === 'slider' || n.role === 'progressbar' ? n.value : null);
      attr('aria-valuetext', c.value_text || (n.role === 'combobox' ? n.value : null));
      attr('aria-level', c.level);
      attr('aria-sort', c.sort);
      attr('aria-multiline', c.multiline || null);
      attr('aria-invalid', c.invalid || null);
      attr('aria-required', c.required || null);
      attr('aria-live', c.live);
      if (n.position) {
        attr('aria-posinset', n.position[0]);
        attr('aria-setsize', n.position[1]);
      }
      el.tabIndex = c.disabled ? -1 : (c.tab_index ?? (n.focusable ? 0 : -1));
      if ('disabled' in el) el.disabled = !!c.disabled;
      el.lang = c.lang || '';
      el.dir = c.direction || 'auto';
      if (el.tagName === 'A') el.href = c.href || '#';
      if (this.editable(el)) {
        if (el.tagName === 'INPUT')
          el.type =
            c.input_type === 'password' ? 'password' : c.input_type === 'email' ? 'email' : 'text';
        el.readOnly = !!c.read_only;
        el.required = !!c.required;
        if (g.editing && el.scrollTop !== g.editing.scroll_y) el.scrollTop = g.editing.scroll_y;
        const value = c.input_type === 'password' ? this.app.edit_value?.(n.id) || '' : n.value;
        if (!this.composing.has(el) && el.value !== value) {
          const a = el.selectionStart,
            b = el.selectionEnd;
          el.value = value;
          if (document.activeElement === el && a !== null && el.type !== 'email')
            el.setSelectionRange(Math.min(a, value.length), Math.min(b, value.length));
        }
        if (
          !this.composing.has(el) &&
          document.activeElement === el &&
          g.editing &&
          el.type !== 'email'
        ) {
          const { anchor_utf16: a, focus_utf16: b } = g.editing;
          if (el.selectionStart !== Math.min(a, b) || el.selectionEnd !== Math.max(a, b))
            el.setSelectionRange(Math.min(a, b), Math.max(a, b), a > b ? 'backward' : 'forward');
        }
      } else if (el.textContent !== n.label) el.textContent = n.label;
      const [x, y, w, h] = g.rect,
        [vx, vy, vw, vh] = g.visible_rect;
      Object.assign(el.style, {
        position: 'absolute',
        left: `${x}px`,
        top: `${y}px`,
        width: `${Math.max(1, w)}px`,
        height: `${Math.max(1, h)}px`,
        zIndex: String(g.paint_order ?? 0),
        pointerEvents: n.focusable || ['list', 'dialog'].includes(n.role) ? 'auto' : 'none',
        clipPath:
          vw > 0 && vh > 0
            ? `inset(${Math.max(0, vy - y)}px ${Math.max(0, x + w - vx - vw)}px ${Math.max(0, y + h - vy - vh)}px ${Math.max(0, vx - x)}px)`
            : 'inset(100%)',
      });
      if (c.modal) modal = el;
    }
    for (const [id, el] of this.elements) {
      if (!seen.has(id)) {
        el.remove();
        this.elements.delete(id);
      } else {
        const owned = ownership.get(id);
        if (owned?.length) el.setAttribute('aria-owns', owned.join(' '));
        else el.removeAttribute('aria-owns');
      }
    }
    const order = inspection.nodes.map((n) => this.elements.get(n.node)).filter(Boolean),
      active = order.indexOf(document.activeElement),
      old = [...this.root.children].indexOf(document.activeElement);
    const arrange = (items, anchor) => {
      for (let i = items.length - 1; i >= 0; i--) {
        if (items[i].nextElementSibling !== anchor) this.root.insertBefore(items[i], anchor);
        anchor = items[i];
      }
    };
    if (active >= 0) {
      arrange(order.slice(0, active), order[active]);
      arrange(order.slice(active + 1), null);
    } else arrange(order, null);
    if (active >= 0 && active !== old) this.reveal(document.activeElement.dataset.node);
    if (modal?.id !== this.lastModal) {
      if (modal) {
        this.restoreFocus = document.activeElement;
        const target = this.controls(modal)[0];
        target?.focus({ preventScroll: true });
      } else if (this.lastModal && this.restoreFocus?.isConnected)
        this.restoreFocus.focus({ preventScroll: true });
      this.lastModal = modal?.id || null;
    }
    this.modal = modal;
    this.flat = flat;
    const allowed = new Set();
    if (modal) {
      const walk = (n) => {
        allowed.add(String(n.id));
        for (const child of n.children || []) walk(child);
      };
      const tree = flat.find((n) => `rustx-${n.id}` === modal.id);
      if (tree) walk(tree);
    }
    for (const el of this.elements.values()) el.inert = !!modal && !allowed.has(el.dataset.node);
    const focused = inspection.focused && this.elements.get(inspection.focused);
    if (focused && focused !== document.activeElement && !focused.inert)
      focused.focus({ preventScroll: true });
  }
  controls(modal = this.modal) {
    let allowed;
    if (modal) {
      allowed = new Set();
      const start = this.flat?.find((n) => `rustx-${n.id}` === modal.id);
      const walk = (n) => {
        allowed.add(String(n.id));
        for (const child of n.children || []) walk(child);
      };
      if (start) walk(start);
    }
    return [...this.root.children].filter(
      (el) => el.tabIndex >= 0 && !el.disabled && (!allowed || allowed.has(el.dataset.node)),
    );
  }
  tab(e) {
    if (e.key !== 'Tab' || e.altKey || e.ctrlKey || e.metaKey) return;
    if (document.activeElement !== document.body && !this.root.contains(document.activeElement))
      return;
    const list = this.controls(),
      i = list.indexOf(document.activeElement);
    let next = i < 0 ? (e.shiftKey ? list.length - 1 : 0) : i + (e.shiftKey ? -1 : 1);
    if (this.modal && list.length) next = (next + list.length) % list.length;
    if (next >= 0 && next < list.length) {
      e.preventDefault();
      list[next].focus({ preventScroll: true });
    }
  }
  dispose() {
    document.removeEventListener('selectionchange', this.onSelection);
    this.elements.clear();
  }
}
