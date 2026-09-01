// Windows'ta konsol penceresi açılmasın: bu bir GUI uygulaması ve arka planda
// da çalışıyor; her açılışta arkada bir konsol parlaması istenmiyor.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    muifly_lib::run()
}
