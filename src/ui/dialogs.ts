let currentBackdrop = null;

export const dialogs = {
  init() {
    /* no-op for now */
  },

  /**
   * @param {{title:string, body:string, actions?:Array<{label:string, primary?:boolean, onClick?:Function}>}} spec
   */
  open(spec) {
    dialogs.close();
    const backdrop = document.createElement('div');
    backdrop.className = 'lc-dialog-backdrop';

    const dlg = document.createElement('div');
    dlg.className = 'lc-dialog';
    const header = document.createElement('div');
    header.className = 'lc-dialog-header';
    header.innerHTML =
      '<span>' + (spec.title || '') + '</span><button class="close" aria-label="Close">✕</button>';
    const body = document.createElement('div');
    body.className = 'lc-dialog-body';
    if (spec.body instanceof Node) {
      body.appendChild(spec.body);
    } else {
      body.textContent = spec.body || '';
      body.style.whiteSpace = 'pre-wrap';
    }
    const actions = document.createElement('div');
    actions.className = 'lc-dialog-actions';
    (spec.actions || [{ label: 'OK', primary: true }]).forEach(function (a) {
      const btn = document.createElement('button');
      btn.className = a.primary ? 'primary' : 'secondary';
      btn.textContent = a.label;
      btn.addEventListener('click', function () {
        if (typeof a.onClick === 'function') a.onClick();
        else dialogs.close();
      });
      actions.appendChild(btn);
    });
    header.querySelector('.close').addEventListener('click', dialogs.close);

    dlg.appendChild(header);
    dlg.appendChild(body);
    dlg.appendChild(actions);
    backdrop.appendChild(dlg);
    document.body.appendChild(backdrop);
    currentBackdrop = backdrop;
  },

  close() {
    if (currentBackdrop && currentBackdrop.parentNode) {
      currentBackdrop.parentNode.removeChild(currentBackdrop);
    }
    currentBackdrop = null;
  },
};
