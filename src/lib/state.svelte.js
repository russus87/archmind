// Stato globale dell'app, basato sui rune di Svelte 5.
// Un singolo oggetto reattivo condiviso da tutti i componenti.

export const store = $state({
  /// Progetto analizzato (modello completo dal backend) o null.
  project: null,
  /// true mentre l'analisi e' in corso.
  loading: false,
  /// Ultimo messaggio di errore, se presente.
  error: "",
  /// Scheda attiva nella sidebar.
  tab: "overview",
});
