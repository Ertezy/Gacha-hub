//! Разбор исполняемого файла Windows ради одной вещи: достать иконку.
//!
//! Написано своими силами и без единой строки `unsafe` намеренно. Обращение к
//! системной функции Windows или сторонняя библиотека сделали бы то же самое,
//! но небезопасный код в них всё равно выполняется — он просто перестаёт быть
//! виден. Проект готовится к публикации, и утверждение «здесь нет
//! небезопасного кода» должно быть правдой, которую любой проверит.
//!
//! Отсюда правило всего модуля: **любое неверное смещение возвращает `None`**.
//! Файл может быть обрезан, повреждён или вообще не быть программой — ответом
//! будет «картинки нет», но никогда не паника.

/// Число из двух байтов, младшим вперёд. `None`, если байтов не хватает.
fn u16_at(b: &[u8], off: usize) -> Option<u16> {
    let s = b.get(off..off.checked_add(2)?)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

/// Число из четырёх байтов, младшим вперёд. `None`, если байтов не хватает.
fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn is_png(data: &[u8]) -> bool {
    data.get(..8).is_some_and(|h| h == PNG_MAGIC)
}

/// Ширина и высота PNG из заголовка IHDR. В PNG числа записаны **старшим
/// байтом вперёд**, в отличие от всего остального в этом файле.
fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    if !is_png(data) {
        return None;
    }
    let w = data.get(16..20)?;
    let h = data.get(20..24)?;
    Some((
        u32::from_be_bytes([w[0], w[1], w[2], w[3]]),
        u32::from_be_bytes([h[0], h[1], h[2], h[3]]),
    ))
}

/// Секция файла: где она лежит в памяти и где в самом файле.
struct Section {
    virtual_address: u32,
    virtual_size: u32,
    raw_offset: u32,
    raw_size: u32,
}

/// Секции файла и адрес таблицы ресурсов.
///
/// Путь по заголовкам: в начале лежит старый заголовок MS-DOS, у него по
/// смещению 0x3C записано, где начинается настоящий заголовок. Дальше подпись
/// «PE\0\0», за ней заголовок с числом секций, за ним необязательный заголовок,
/// в конце которого — таблица каталогов; второй каталог и есть ресурсы.
fn sections_and_resources(b: &[u8]) -> Option<(Vec<Section>, u32)> {
    if b.get(..2)? != b"MZ" {
        return None;
    }
    let pe = u32_at(b, 0x3C)? as usize;
    if b.get(pe..pe.checked_add(4)?)? != b"PE\0\0" {
        return None;
    }

    let coff = pe.checked_add(4)?;
    let section_count = u16_at(b, coff.checked_add(2)?)? as usize;
    let optional_size = u16_at(b, coff.checked_add(16)?)? as usize;
    let optional = coff.checked_add(20)?;

    // 0x10b — 32-разрядный файл, 0x20b — 64-разрядный. Отличаются они только
    // тем, где начинается таблица каталогов.
    let directories = match u16_at(b, optional)? {
        0x10b => optional.checked_add(96)?,
        0x20b => optional.checked_add(112)?,
        _ => return None,
    };
    // Второй каталог по счёту (нумерация с нуля) — таблица ресурсов.
    let resource_rva = u32_at(b, directories.checked_add(2 * 8)?)?;
    if resource_rva == 0 {
        return None;
    }

    let table = optional.checked_add(optional_size)?;
    let mut sections = Vec::with_capacity(section_count.min(96));
    for i in 0..section_count {
        let s = table.checked_add(i.checked_mul(40)?)?;
        sections.push(Section {
            virtual_size: u32_at(b, s.checked_add(8)?)?,
            virtual_address: u32_at(b, s.checked_add(12)?)?,
            raw_size: u32_at(b, s.checked_add(16)?)?,
            raw_offset: u32_at(b, s.checked_add(20)?)?,
        });
    }
    Some((sections, resource_rva))
}

/// Адрес в памяти переводится в смещение внутри файла: находим секцию, в
/// которую адрес попадает, и сдвигаемся на столько же от её начала в файле.
fn rva_to_file(sections: &[Section], rva: u32) -> Option<usize> {
    for s in sections {
        let end = s.virtual_address.checked_add(s.virtual_size)?;
        if rva >= s.virtual_address && rva < end {
            let inside = rva.checked_sub(s.virtual_address)?;
            if inside >= s.raw_size {
                return None;
            }
            return s.raw_offset.checked_add(inside).map(|v| v as usize);
        }
    }
    None
}

/// Тип ресурса «картинка иконки».
const RT_ICON: u32 = 3;

/// Общий предел числа шагов на весь обход дерева ресурсов, а не на один
/// узел. Ограничение в `children` — только на одну запись; но записи трёх
/// уровней (тип, номер, язык) могут все указывать на один и тот же узел
/// следующего уровня, и тогда работа перемножается, а не складывается: три
/// уровня по 4096 записей — это уже около семидесяти миллиардов шагов при
/// файле в сотню килобайт. У настоящих программ иконок единицы, у самых
/// богатых — десятки, так что несколько сотен шагов на весь обход — предел
/// с большим запасом.
const MAX_RESOURCE_STEPS: usize = 512;

/// Собирает адреса и размеры всех картинок иконок.
///
/// Ресурсы лежат деревом ровно из трёх уровней: тип, номер, язык. У каждого
/// узла сначала шестнадцать байт заголовка, потом записи по восемь байт.
/// Старший бит в записи означает «дальше ещё узел», иначе это лист.
fn collect_icons(b: &[u8], sections: &[Section], resource_rva: u32) -> Vec<(u32, u32)> {
    let mut found = Vec::new();
    let Some(root) = rva_to_file(sections, resource_rva) else {
        return found;
    };

    // Бюджет общий на весь обход и передаётся во все вызовы `children`, а не
    // заводится заново на каждом уровне — иначе повторно используемый узел
    // обошёл бы поштучный предел, размножая работу через уровни дерева.
    let mut budget = MAX_RESOURCE_STEPS;

    for (type_id, type_off) in children(b, root, root, &mut budget) {
        if type_id != RT_ICON {
            continue;
        }
        for (_, name_off) in children(b, root, type_off, &mut budget) {
            for (_, lang_off) in children(b, root, name_off, &mut budget) {
                // Лист: адрес данных и их размер.
                let (Some(rva), Some(size)) = (u32_at(b, lang_off), u32_at(b, lang_off + 4)) else {
                    continue;
                };
                found.push((rva, size));
            }
        }
    }
    found
}

/// Записи одного узла дерева. Возвращает пары «номер, смещение».
///
/// Для узла отдаётся смещение следующего узла, для листа — смещение записи с
/// данными. Различаются они старшим битом, и вызывающий знает по уровню, что
/// именно получил.
///
/// `budget` общий на весь обход (см. `MAX_RESOURCE_STEPS`): каждая
/// рассмотренная запись стоит одну единицу, и как только он заканчивается,
/// узел возвращает уже собранное и дальше не читает — независимо от того,
/// сколько раз до этого узла уже добирались с других ветвей дерева.
fn children(b: &[u8], root: usize, node: usize, budget: &mut usize) -> Vec<(u32, usize)> {
    let mut out = Vec::new();
    let Some(named) = u16_at(b, node.saturating_add(12)) else {
        return out;
    };
    let Some(by_id) = u16_at(b, node.saturating_add(14)) else {
        return out;
    };
    // Переполнение здесь безопасно: сложение упирается в предел, а чтение по
    // запредельному смещению всё равно вернёт None и цикл прервётся.
    let total = named as usize + by_id as usize;
    for i in 0..total.min(4096) {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        let entry = node.saturating_add(16).saturating_add(i.saturating_mul(8));
        let Some(id) = u32_at(b, entry) else { break };
        let Some(offset) = u32_at(b, entry.saturating_add(4)) else { break };
        out.push((id, root.saturating_add((offset & 0x7FFF_FFFF) as usize)));
    }
    out
}

/// Картинка размером 256×256 в формате PNG, если она есть внутри файла.
///
/// **Правило «взять самый большой ресурс» неверно, и это проверено на живых
/// играх.** У четырёх игр из пяти самый большой ресурс действительно оказался
/// нужным PNG, а у Wuthering Waves самый большой — картинка в старом несжатом
/// формате на 264 КБ, тогда как нужный PNG лежит отдельно и **меньше по
/// размеру**. Поэтому ищем именно PNG именно нужного размера.
pub fn icon_png_256(bytes: &[u8]) -> Option<Vec<u8>> {
    let (sections, resource_rva) = sections_and_resources(bytes)?;
    for (rva, size) in collect_icons(bytes, &sections, resource_rva) {
        let Some(start) = rva_to_file(&sections, rva) else {
            continue;
        };
        let Some(end) = start.checked_add(size as usize) else {
            continue;
        };
        let Some(data) = bytes.get(start..end) else {
            continue;
        };
        if png_size(data) == Some((256, 256)) {
            return Some(data.to_vec());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u32_at_reads_a_little_endian_number() {
        let b = [0x78, 0x56, 0x34, 0x12];
        assert_eq!(u32_at(&b, 0), Some(0x12345678));
    }

    #[test]
    fn u32_at_refuses_to_read_past_the_end() {
        // Смещение за границей не должно ронять программу: весь разбор
        // построен на том, что кривой файл даёт None, а не панику.
        let b = [0x01, 0x02];
        assert_eq!(u32_at(&b, 0), None);
        assert_eq!(u32_at(&b, 100), None);
    }

    #[test]
    fn u16_at_refuses_to_read_past_the_end() {
        let b = [0x01];
        assert_eq!(u16_at(&b, 0), None);
    }

    #[test]
    fn png_signature_is_recognised() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(is_png(&png));
        assert!(!is_png(&[0x28, 0x00, 0x00, 0x00]));
        assert!(!is_png(&[]));
    }

    #[test]
    fn png_size_reads_the_header() {
        // Подпись PNG, длина куска, слово IHDR, затем ширина и высота
        // четырьмя байтами каждая, старшим байтом вперёд.
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        b.extend_from_slice(&[0, 0, 0, 13]);
        b.extend_from_slice(b"IHDR");
        b.extend_from_slice(&256u32.to_be_bytes());
        b.extend_from_slice(&256u32.to_be_bytes());
        assert_eq!(png_size(&b), Some((256, 256)));
    }

    #[test]
    fn png_size_refuses_a_truncated_header() {
        let b = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(png_size(&b), None);
    }

    #[test]
    fn garbage_input_does_not_panic() {
        // Главное свойство модуля: что бы ни пришло на вход, ответ либо
        // картинка, либо None, но никогда не паника.
        assert_eq!(icon_png_256(&[]), None);
        assert_eq!(icon_png_256(&[0u8; 3]), None);
        assert_eq!(icon_png_256(&[0xFFu8; 1024]), None);
        let almost = {
            let mut v = vec![0u8; 64];
            v[0] = b'M';
            v[1] = b'Z';
            v[0x3C] = 200; // указывает за пределы файла
            v
        };
        assert_eq!(icon_png_256(&almost), None);
    }

    /// Строит один узел дерева ресурсов: 16 байт заголовка (важно только
    /// число записей по смещению 14) и `count` одинаковых записей по 8 байт
    /// — пара «номер, смещение», где смещение уже отсчитано от корня.
    fn resource_node(count: u16, id: u32, offset: u32) -> Vec<u8> {
        let mut node = vec![0u8; 16];
        node[14..16].copy_from_slice(&count.to_le_bytes());
        for _ in 0..count {
            node.extend_from_slice(&id.to_le_bytes());
            node.extend_from_slice(&offset.to_le_bytes());
        }
        node
    }

    #[test]
    fn collect_icons_bounds_total_work_even_when_the_tree_reuses_nodes() {
        // Подлог: записи каждого уровня ссылаются не на разные узлы, а на
        // один и тот же. Дерево из трёх уровней по 32 записи разворачивается
        // не в 96 действий, а в 32*32*32 = 32768 — множитель, а не сумма.
        // У настоящих файлов такого повторного использования не бывает, но
        // формат этого не запрещает, а обход не имел общего предела.
        const N: u16 = 32;

        let root_len = 16 + N as u32 * 8;
        let name_off = root_len;
        let name_len = 16 + N as u32 * 8;
        let lang_off = name_off + name_len;
        let lang_len = 16 + N as u32 * 8;
        let leaf_off = lang_off + lang_len;

        let mut b = resource_node(N, RT_ICON, name_off);
        b.extend(resource_node(N, 0, lang_off));
        b.extend(resource_node(N, 0, leaf_off));
        b.extend_from_slice(&0u32.to_le_bytes()); // rva листа
        b.extend_from_slice(&8u32.to_le_bytes()); // размер листа

        let sections = [Section {
            virtual_address: 0,
            virtual_size: b.len() as u32,
            raw_offset: 0,
            raw_size: b.len() as u32,
        }];

        let found = collect_icons(&b, &sections, 0);
        assert!(
            found.len() < MAX_RESOURCE_STEPS,
            "обход должен быть ограничен общим бюджетом на весь разбор, а не \
             размножаться по числу совпавших узлов: получено {} записей",
            found.len()
        );
    }
}
