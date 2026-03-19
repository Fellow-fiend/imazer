#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
}

impl Lang {
    pub fn display_name(self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Ru => "RU",
        }
    }
}

pub fn t(lang: Lang, key: &'static str) -> &'static str {
    match lang {
        Lang::En => en(key),
        Lang::Ru => ru(key),
    }
}

fn en(key: &str) -> &str {
    match key {
        "toolbar_hint" => "Drop images here or use Select Files.",
        "resize_images" => "Resize Images",
        "batch_settings" => "Batch Settings",
        "width" => "Width",
        "height" => "Height",
        "pixels" => "Pixels",
        "percent" => "Percent",
        "keep_ratio" => "Lock aspect ratio",
        "auto_output" => "Auto output folder",
        "output_folder" => "Output folder",
        "output_auto_hint" => "Auto: source folder (or source/resized for mixed folders)",
        "choose_folder" => "Choose Folder",
        "clear_output" => "Clear Custom",
        "select_files" => "Select Files",
        "remove_selected" => "Remove Selected",
        "clear_all" => "Clear All",
        "loaded_images" => "Loaded images",
        "image_queue" => "Image Queue",
        "preview" => "Preview",
        "select_for_preview" => "Select an image to preview",
        "log" => "Log",
        _ => key,
    }
}

fn ru(key: &str) -> &str {
    match key {
        "toolbar_hint" => "Перетащите изображения сюда или нажмите «Выбрать файлы».",
        "resize_images" => "Изменить размер",
        "batch_settings" => "Пакетные настройки",
        "width" => "Ширина",
        "height" => "Высота",
        "pixels" => "Пиксели",
        "percent" => "Проценты",
        "keep_ratio" => "Сохранять пропорции",
        "auto_output" => "Автовыбор папки вывода",
        "output_folder" => "Папка вывода",
        "output_auto_hint" => "Авто: папка источника (или source/resized для разных папок)",
        "choose_folder" => "Выбрать папку",
        "clear_output" => "Очистить путь",
        "select_files" => "Выбрать файлы",
        "remove_selected" => "Удалить выбранное",
        "clear_all" => "Очистить всё",
        "loaded_images" => "Загружено изображений",
        "image_queue" => "Очередь изображений",
        "preview" => "Предпросмотр",
        "select_for_preview" => "Выберите изображение для предпросмотра",
        "log" => "Журнал",
        _ => key,
    }
}
