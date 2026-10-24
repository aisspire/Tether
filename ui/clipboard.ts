// Supply the text promise during the click so clipboard permission keeps its
// user activation while the backend resolves SSH configuration.
export async function copyGeneratedText(load: () => Promise<string>) {
  const text = load()
  try {
    if (typeof ClipboardItem !== 'undefined' && navigator.clipboard.write) {
      const data = text.then(value => new Blob([value], { type: 'text/plain' }))
      // Permission may be denied before the browser consumes the supplied data.
      void data.catch(() => {})
      await navigator.clipboard.write([new ClipboardItem({
        'text/plain': data,
      })])
    } else {
      await navigator.clipboard.writeText(await text)
    }
  } catch (error) {
    // Preserve a useful backend error if the clipboard reports only that the
    // promised data failed. Also consume the promise after permission denial.
    await text
    throw error
  }
}
