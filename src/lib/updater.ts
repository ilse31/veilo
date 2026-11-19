import { check, type Update } from '@tauri-apps/plugin-updater'
import { ask } from '@tauri-apps/plugin-dialog'
import { relaunch } from '@tauri-apps/plugin-process'

export async function downloadAndInstall(
  update: Update,
  onProgress?: (received: number, total: number | undefined) => void,
): Promise<void> {
  let received = 0
  let total: number | undefined
  await update.downloadAndInstall((ev) => {
    if (ev.event === 'Started') total = ev.data.contentLength
    else if (ev.event === 'Progress') {
      received += ev.data.chunkLength
      onProgress?.(received, total)
    }
  })
}

// Silent check on startup — prompts only when an update actually exists.
export async function autoCheckUpdate(): Promise<void> {
  try {
    const update = await check()
    if (!update) return
    const ok = await ask(`Veilo ${update.version} tersedia. Unduh dan pasang sekarang?`, {
      title: 'Pembaruan Veilo',
      kind: 'info',
      okLabel: 'Perbarui',
      cancelLabel: 'Nanti',
    })
    if (!ok) return
    await downloadAndInstall(update)
    const restart = await ask('Pembaruan terpasang. Mulai ulang Veilo sekarang?', {
      title: 'Pembaruan Veilo',
      okLabel: 'Mulai ulang',
      cancelLabel: 'Nanti',
    })
    if (restart) await relaunch()
  } catch { /* offline / dev — abaikan */ }
}
