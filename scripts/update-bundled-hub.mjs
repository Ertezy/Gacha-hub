// Перед выпуском приложения: заменить файл из комплекта свежим файлом сборщика.
import { writeFileSync } from "node:fs";

const SOURCE = "https://ertezy.github.io/Kitsudock-data/hub.json";
const TARGET = "src-tauri/resources/hub.json";

const res = await fetch(SOURCE, { headers: { "User-Agent": "KitsudockRelease/1.0" } });
if (!res.ok) throw new Error(`файл не скачался: ${res.status} ${SOURCE}`);
const hub = await res.json();
if (hub.version !== 2 || !Array.isArray(hub.games) || hub.games.length === 0 || !Array.isArray(hub.codes) || !Array.isArray(hub.banners) || !Array.isArray(hub.videos)) {
  throw new Error("скачанный файл не похож на хаб версии 2");
}
writeFileSync(TARGET, `${JSON.stringify(hub, null, 2)}\n`);
console.log(`${TARGET}: коды ${hub.codes.length}, баннеры ${hub.banners.length}, видео ${hub.videos.length}`);
