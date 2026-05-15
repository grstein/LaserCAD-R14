/**
 * Dispara o download de uma string como arquivo (file://-friendly).
 * @param {string} filename
 * @param {string} content
 * @param {string} [mime='image/svg+xml']
 */
function download(filename, content, mime?) {
  const blob = new Blob([content], { type: mime || 'image/svg+xml;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  setTimeout(function () {
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, 0);
}

export const fileDownload = { download: download };
