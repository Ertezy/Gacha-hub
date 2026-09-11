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
        // `continue`, а не `?`: переполнение здесь испорчено именно у этой
        // секции, а не у файла целиком. Нужный адрес может лежать в одной из
        // следующих секций, и она вполне может оказаться исправной — выходить
        // из всего поиска из-за одной плохой записи нельзя.
        let Some(end) = s.virtual_address.checked_add(s.virtual_size) else {
            continue;
        };
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

/// Тип ресурса «группа иконок». Группа — это оглавление: какие размеры есть и
/// под какими номерами лежат их картинки.
const RT_GROUP_ICON: u32 = 14;

/// Общий предел числа шагов на весь обход дерева ресурсов, а не на один
/// узел. Ограничение в `children` — только на одну запись; но записи трёх
/// уровней (тип, номер, язык) могут все указывать на один и тот же узел
/// следующего уровня, и тогда работа перемножается, а не складывается: три
/// уровня по 4096 записей — это уже около семидесяти миллиардов шагов при
/// файле в сотню килобайт. У настоящих программ иконок единицы, у самых
/// богатых — десятки, так что несколько сотен шагов на весь обход — предел
/// с большим запасом.
const MAX_RESOURCE_STEPS: usize = 512;

/// Ресурс: под каким номером лежит и где искать его содержимое.
struct Resource {
    id: u32,
    rva: u32,
    size: u32,
}

/// Собирает картинки иконок и их оглавления **за один обход**.
///
/// Ресурсы лежат деревом ровно из трёх уровней: тип, номер, язык. У каждого
/// узла сначала шестнадцать байт заголовка, потом записи по восемь байт.
/// Старший бит в записи означает «дальше ещё узел», иначе это лист.
///
/// Обход один на оба типа, а не по обходу на каждый, потому что бюджет шагов
/// общий (см. `MAX_RESOURCE_STEPS`). Два прохода дали бы по полному бюджету
/// каждому — то есть тихо удвоили бы границу, ради которой он заведён.
fn collect_resources(
    b: &[u8],
    sections: &[Section],
    resource_rva: u32,
) -> (Vec<Resource>, Vec<Resource>) {
    let mut icons = Vec::new();
    let mut groups = Vec::new();
    let Some(root) = rva_to_file(sections, resource_rva) else {
        return (icons, groups);
    };

    // Бюджет общий на весь обход и передаётся во все вызовы `children`, а не
    // заводится заново на каждом уровне — иначе повторно используемый узел
    // обошёл бы поштучный предел, размножая работу через уровни дерева.
    let mut budget = MAX_RESOURCE_STEPS;

    for (type_id, type_off) in children(b, root, root, &mut budget) {
        let into = match type_id {
            RT_ICON => &mut icons,
            RT_GROUP_ICON => &mut groups,
            _ => continue,
        };
        for (name_id, name_off) in children(b, root, type_off, &mut budget) {
            for (_, lang_off) in children(b, root, name_off, &mut budget) {
                // Лист: адрес данных и их размер. `checked_add`, как и везде
                // в модуле — единственное место, где раньше складывали
                // смещение напрямую (правило заявлено в заголовке файла).
                let Some(size_off) = lang_off.checked_add(4) else {
                    continue;
                };
                let (Some(rva), Some(size)) = (u32_at(b, lang_off), u32_at(b, size_off)) else {
                    continue;
                };
                into.push(Resource { id: name_id, rva, size });
            }
        }
    }
    (icons, groups)
}

/// Содержимое ресурса.
fn resource_bytes<'a>(b: &'a [u8], sections: &[Section], res: &Resource) -> Option<&'a [u8]> {
    let start = rva_to_file(sections, res.rva)?;
    let end = start.checked_add(res.size as usize)?;
    b.get(start..end)
}

/// Записи оглавления группы: ширина картинки и номер ресурса, в котором она
/// лежит.
///
/// Оглавление устроено так: шесть байт заголовка, дальше записи по
/// четырнадцать. Ширина занимает **один байт**, поэтому число 256 в него не
/// помещается и записывается нулём.
fn group_members(dir: &[u8]) -> Vec<(u32, u32)> {
    let Some(count) = u16_at(dir, 4) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..count as usize {
        let Some(entry) = i.checked_mul(14).and_then(|o| o.checked_add(6)) else {
            break;
        };
        let Some(width) = dir.get(entry).copied() else {
            break;
        };
        let Some(id) = u16_at(dir, entry.saturating_add(12)) else {
            break;
        };
        out.push((if width == 0 { 256 } else { width as u32 }, id as u32));
    }
    out
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

/// Картинки иконки в том порядке, в каком их стоит пробовать: самая крупная
/// первой. Пусто, если файл не разобрался или иконок в нём нет.
///
/// **Выбирается группа, а не отдельная картинка, и это главное здесь.** В
/// одном файле групп может быть несколько, и Windows показывает ту, у которой
/// наименьший номер. У Wuthering Waves групп ровно две: под номером 101 лежит
/// настоящая иконка игры, под номером 123 — шаблонный логотип движка,
/// оставшийся от сборки. Перебор картинок подряд, без оглядки на группы,
/// приводил именно ко второй.
///
/// Заодно отпало правило «взять PNG размером 256»: нужная картинка Wuthering
/// Waves записана в старом формате, и по этому правилу не подходила вовсе.
/// Размер берётся из оглавления, разбирать саму картинку ради него не нужно.
pub fn icon_candidates(bytes: &[u8]) -> Vec<Vec<u8>> {
    let Some((sections, resource_rva)) = sections_and_resources(bytes) else {
        return Vec::new();
    };
    candidates_in(bytes, &sections, resource_rva)
}

/// Выбор без разбора заголовков файла: секции и адрес таблицы уже найдены.
/// Вынесено отдельно ради тестов — собрать дерево ресурсов в памяти куда
/// проще, чем целый исполняемый файл.
fn candidates_in(b: &[u8], sections: &[Section], resource_rva: u32) -> Vec<Vec<u8>> {
    let (icons, groups) = collect_resources(b, sections, resource_rva);

    // Оглавления нет вовсе — редкий случай, выбирать не из чего. Берём все
    // картинки, начиная с самой тяжёлой: без оглавления размер картинки
    // иначе как разбором её самой не узнать, а вес — приемлемая замена.
    let Some(group) = groups.iter().min_by_key(|g| g.id) else {
        let mut loose: Vec<&Resource> = icons.iter().collect();
        loose.sort_by_key(|r| std::cmp::Reverse(r.size));
        return loose
            .into_iter()
            .filter_map(|r| resource_bytes(b, sections, r).map(<[u8]>::to_vec))
            .collect();
    };

    let Some(dir) = resource_bytes(b, sections, group) else {
        return Vec::new();
    };
    let mut members = group_members(dir);
    // Сортировка устойчивая: при равной ширине порядок остаётся тем, что задан
    // в оглавлении. У Honkai: Star Rail две картинки по 256 подряд, и от этого
    // зависит, какая из них станет иконкой.
    members.sort_by_key(|(width, _)| std::cmp::Reverse(*width));
    members
        .into_iter()
        .filter_map(|(_, id)| icons.iter().find(|r| r.id == id))
        .filter_map(|r| resource_bytes(b, sections, r).map(<[u8]>::to_vec))
        .collect()
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
    fn rva_to_file_skips_a_section_with_overflowing_bounds_and_keeps_looking() {
        // Первая секция испорчена: адрес плюс размер переполняют u32. Раньше
        // `?` в `rva_to_file` прерывал весь поиск на этом месте, и вторая,
        // исправная секция даже не проверялась — хотя искомый адрес лежит
        // именно в ней.
        let sections = [
            Section {
                virtual_address: u32::MAX - 10,
                virtual_size: 1000,
                raw_offset: 0,
                raw_size: 100,
            },
            Section {
                virtual_address: 0x1000,
                virtual_size: 0x100,
                raw_offset: 0x400,
                raw_size: 0x100,
            },
        ];
        assert_eq!(rva_to_file(&sections, 0x1010), Some(0x410));
    }

    #[test]
    fn garbage_input_does_not_panic() {
        // Главное свойство модуля: что бы ни пришло на вход, ответ либо
        // картинки, либо пусто, но никогда не паника.
        assert!(icon_candidates(&[]).is_empty());
        assert!(icon_candidates(&[0u8; 3]).is_empty());
        assert!(icon_candidates(&[0xFFu8; 1024]).is_empty());
        let almost = {
            let mut v = vec![0u8; 64];
            v[0] = b'M';
            v[1] = b'Z';
            v[0x3C] = 200; // указывает за пределы файла
            v
        };
        assert!(icon_candidates(&almost).is_empty());
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

        let (found, _) = collect_resources(&b, &sections, 0);
        assert!(
            found.len() < MAX_RESOURCE_STEPS,
            "обход должен быть ограничен общим бюджетом на весь разбор, а не \
             размножаться по числу совпавших узлов: получено {} записей",
            found.len()
        );
    }

    /// Узел дерева из перечисленных записей «номер, смещение от корня».
    fn node(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut out = vec![0u8; 16];
        out[14..16].copy_from_slice(&(entries.len() as u16).to_le_bytes());
        for (id, offset) in entries {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&offset.to_le_bytes());
        }
        out
    }

    /// Лист: адрес содержимого и его длина.
    fn leaf(rva: u32, size: u32) -> Vec<u8> {
        let mut out = vec![0u8; 16];
        out[0..4].copy_from_slice(&rva.to_le_bytes());
        out[4..8].copy_from_slice(&size.to_le_bytes());
        out
    }

    /// Оглавление группы из записей «ширина, номер картинки».
    fn group_dir(members: &[(u8, u16)]) -> Vec<u8> {
        let mut out = vec![0u8; 6];
        out[2..4].copy_from_slice(&1u16.to_le_bytes()); // тип: иконка
        out[4..6].copy_from_slice(&(members.len() as u16).to_le_bytes());
        for (width, id) in members {
            let mut entry = vec![0u8; 14];
            entry[0] = *width;
            entry[12..14].copy_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&entry);
        }
        out
    }

    /// Секция, отображающая адреса в файл один в один.
    fn whole(b: &[u8]) -> [Section; 1] {
        [Section {
            virtual_address: 0,
            virtual_size: b.len() as u32,
            raw_offset: 0,
            raw_size: b.len() as u32,
        }]
    }

    /// Дерево ресурсов с двумя картинками (номера 1 и 2, содержимое `REAL` и
    /// `LOGO`) и двумя группами. Группа 123 записана **первой**, группа 101
    /// второй — как в файле Wuthering Waves, где порядок в дереве и порядок
    /// по номеру расходятся.
    fn tree(dir_123: &[(u8, u16)], dir_101: &[(u8, u16)]) -> Vec<u8> {
        let (icon1, icon2) = (&b"REAL"[..], &b"LOGO"[..]);
        let (d123, d101) = (group_dir(dir_123), group_dir(dir_101));
        let p_icon1 = 256u32;
        let p_icon2 = p_icon1 + icon1.len() as u32;
        let p_123 = p_icon2 + icon2.len() as u32;
        let p_101 = p_123 + d123.len() as u32;

        let mut b = Vec::new();
        b.extend(node(&[(RT_ICON, 32), (RT_GROUP_ICON, 112)])); // корень
        b.extend(node(&[(1, 64), (2, 88)])); // тип «картинка»
        b.extend(node(&[(0, 192)])); // имя картинки 1
        b.extend(node(&[(0, 208)])); // имя картинки 2
        b.extend(node(&[(123, 144), (101, 168)])); // тип «группа»
        b.extend(node(&[(0, 224)])); // имя группы 123
        b.extend(node(&[(0, 240)])); // имя группы 101
        b.extend(leaf(p_icon1, icon1.len() as u32));
        b.extend(leaf(p_icon2, icon2.len() as u32));
        b.extend(leaf(p_123, d123.len() as u32));
        b.extend(leaf(p_101, d101.len() as u32));
        assert_eq!(b.len(), p_icon1 as usize, "узлы и листы должны занять ровно 256 байт");
        b.extend_from_slice(icon1);
        b.extend_from_slice(icon2);
        b.extend_from_slice(&d123);
        b.extend_from_slice(&d101);
        b
    }

    #[test]
    fn the_group_with_the_lowest_number_wins_over_the_one_listed_first() {
        // Это и есть случай Wuthering Waves: в файле две группы, первой в
        // дереве лежит 123 с логотипом движка, а показывать нужно 101 с
        // иконкой игры. Перебор картинок подряд выбирал логотип.
        let b = tree(&[(0, 2)], &[(0, 1)]);
        assert_eq!(candidates_in(&b, &whole(&b), 0), vec![b"REAL".to_vec()]);
    }

    #[test]
    fn inside_a_group_the_widest_picture_comes_first() {
        // Ширина берётся из оглавления, а не из самой картинки: разбирать её
        // ради размера не нужно, и для старого формата это важно — там размер
        // лежит в другом месте, чем у PNG.
        let b = tree(&[(0, 2)], &[(32, 2), (0, 1)]);
        assert_eq!(
            candidates_in(&b, &whole(&b), 0),
            vec![b"REAL".to_vec(), b"LOGO".to_vec()],
            "картинка на 256 точек должна опередить картинку на 32"
        );
    }

    #[test]
    fn without_any_group_the_heaviest_picture_comes_first() {
        // Запасной путь: оглавления нет, размер узнать неоткуда, и вес
        // остаётся единственным признаком.
        let mut b = Vec::new();
        b.extend(node(&[(RT_ICON, 24)])); // корень: только картинки
        b.extend(node(&[(1, 56), (2, 80)])); // тип «картинка»
        b.extend(node(&[(0, 104)])); // имя картинки 1
        b.extend(node(&[(0, 120)])); // имя картинки 2
        b.extend(leaf(136, 2)); // картинка 1 — два байта
        b.extend(leaf(138, 6)); // картинка 2 — шесть байт
        assert_eq!(b.len(), 136, "узлы и листы должны занять ровно 136 байт");
        b.extend_from_slice(b"XXYYYYYY");
        assert_eq!(
            candidates_in(&b, &whole(&b), 0),
            vec![b"YYYYYY".to_vec(), b"XX".to_vec()]
        );
    }
}
