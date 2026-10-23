class PaletteStore {
  open = $state(false);
  initialQuery = $state("");
  refreshSerial = $state(0);

  show(query = "") {
    this.initialQuery = query;
    this.open = true;
  }

  hide() {
    this.open = false;
  }

  toggle() {
    this.open ? this.hide() : this.show();
  }

  // Глобальный «обновить»: экраны подписываются на счётчик и перезагружают данные.
  requestRefresh() {
    this.refreshSerial += 1;
  }
}

export const palette = new PaletteStore();
