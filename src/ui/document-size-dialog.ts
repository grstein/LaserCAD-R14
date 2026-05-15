import { state } from '@/app/state.js';
import { camera } from '@/render/camera.js';
import { autosave } from '@/io/autosave.js';
import { dialogs } from '@/ui/dialogs.js';

function buildBody(currentW, currentH) {
  const wrap = document.createElement('div');
  wrap.style.display = 'flex';
  wrap.style.flexDirection = 'column';
  wrap.style.gap = '12px';
  wrap.style.minWidth = '260px';

  function field(labelText, value) {
    const row = document.createElement('label');
    row.style.display = 'flex';
    row.style.flexDirection = 'column';
    row.style.gap = '4px';
    row.style.fontSize = 'var(--font-sm)';
    const lbl = document.createElement('span');
    lbl.textContent = labelText;
    const input = document.createElement('input');
    input.type = 'number';
    input.min = '1';
    input.max = '2000';
    input.step = '1';
    input.value = String(value);
    input.style.padding = '6px 8px';
    input.style.background = 'var(--bg-elevated)';
    input.style.color = 'var(--text-primary)';
    input.style.border = '1px solid var(--border-strong)';
    input.style.borderRadius = '4px';
    input.style.fontFamily = 'var(--font-mono)';
    row.appendChild(lbl);
    row.appendChild(input);
    return { row: row, input: input };
  }

  const wField = field('Width (mm)', currentW);
  const hField = field('Height (mm)', currentH);
  wrap.appendChild(wField.row);
  wrap.appendChild(hField.row);

  return { body: wrap, widthInput: wField.input, heightInput: hField.input };
}

function parseDim(value) {
  const n = parseFloat(value);
  if (!isFinite(n) || n <= 0) return null;
  return Math.max(1, Math.min(2000, n));
}

export const documentSizeDialog = {
  open() {
    const built = buildBody(state.documentBounds.w, state.documentBounds.h);
    const { widthInput, heightInput } = built;

    function flagInvalid(input) {
      input.style.outline = '1px solid #FF2D7A';
    }
    function clearInvalid(input) {
      input.style.outline = '';
    }

    function submit() {
      const w = parseDim(widthInput.value);
      const h = parseDim(heightInput.value);
      if (w == null) {
        flagInvalid(widthInput);
      } else {
        clearInvalid(widthInput);
      }
      if (h == null) {
        flagInvalid(heightInput);
      } else {
        clearInvalid(heightInput);
      }
      if (w == null || h == null) return;
      state.setDocumentBounds({ w: w, h: h });
      if (autosave && autosave.saveNow) {
        autosave.saveNow();
      }
      if (camera && camera.zoomExtents) {
        camera.zoomExtents();
      }
      dialogs.close();
    }

    [widthInput, heightInput].forEach(function (inp) {
      inp.addEventListener('input', function () {
        clearInvalid(inp);
      });
      inp.addEventListener('keydown', function (e) {
        if (e.key === 'Enter') {
          e.preventDefault();
          submit();
        }
      });
    });

    dialogs.open({
      title: 'Document size',
      body: built.body,
      actions: [
        {
          label: 'Cancel',
          onClick: function () {
            dialogs.close();
          },
        },
        { label: 'OK', primary: true, onClick: submit },
      ],
    });

    setTimeout(function () {
      widthInput.focus();
      widthInput.select();
    }, 0);
  },
};
