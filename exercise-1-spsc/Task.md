## Lernziele

Speicherkonsistenz in Anwesenheit von Nebenläufigkeit
lockfreie Synchronisation mit Atomics
Nachrichtenkanäle

## Überblick

In der Übung haben wir verschiedene Paradigmen für die Inter-Thread-Kommunikation sowie deren Unterstützung durch die Rust-Standardbibliothek besprochen. Ein Ansatz war der nachrichtenbasierte Austausch von Informationen über einen Kanal, der durch die Standardbibliothek im Namensraum std::sync::mpsc für mehrere Produzenten und einen Konsumenten (Multiple Producer, Single Consumer) umgesetzt ist.

Ihre Aufgabe besteht darin, eine spezialisierte Version von MPSC, nämlich SPSC (Single Producer, Single Consumer), lockfrei in Rust zu implementieren, zu testen und mit der generelleren Variante aus der Standardbibliothek zu vergleichen. Dafür geben wir Ihnen die Schnittstelle in Form einer Cargo-Projektdatei vor. Bitte achten Sie darauf, dass Sender und Empfänger in unterschiedlichen Threads existieren und sämtliche versendeten Nachrichten genau in der Absendereihenfolge ankommen.

## Schnittstelle

Bitte setzen Sie die folgende Schnittstelle ohne Nutzung blockierender Synchronisationsprimitive um:

```
pub struct Producer<T: Send> {...}
pub struct Consumer<T: Send> {...}

pub struct SendError<T>(pub T);
pub struct RecvError;

impl<T: Send> Producer<T> {
»   pub fn send(&self, val: T) -> Result<(), SendError<T>> {...}
}

impl<T: Send> Consumer<T> {
»   pub fn recv(&self) -> Result<T, RecvError> {...}
}

impl<T: Send> Iterator for Consumer<T> {
»   type Item = T;
»   fn next(&mut self) -> Option<Self::Item> {...}
}

unsafe impl<T: Send> Send for Producer<T> {}
unsafe impl<T: Send> Send for Consumer<T> {}

pub fn channel<T: Send>() -> (Producer<T>, Consumer<T>) {...}
```

## Hinweise

Sie dürfen prinzipiell die Rust-Standardbibliothek, insbesondere Atomics aus std::sync::atomic nutzen. Davon ausgenommen sind MPSC-Kanäle und blockierende Synchronisationsprimitive wie Mutex und RwLock. Wir geben Ihnen eine Cargo-Projektdatei vor, deren Quellcode Sie beliebig, auch mit unsafe-Code, erweitern dürfen. Bitte sehen Sie von Modifikationen der [dependencies]-Sektion in der Cargo.toml-Datei ab, es ist Ihnen jedoch gestattet weitere [dev-dependencies] zu Test- und Benchmarkzwecken mit aufzunehmen.

Wir haben eine kleine Testsuite mit grundlegenden Tests sowie einen Benchmark für den Laufzeitvergleich zwischen Ihrer Lösung und der Implementation aus der Rust-Standardbibliothek für Sie vorbereitet. Testsuite und Benchmark dürfen Sie erweitern und können mittels cargo test bzw. cargo bench ausgeführt werden. Die Benchmark-Bibliothek produziert neben einfachen textuellen Statistiken auch grafische Ausgaben im html-Format, die Sie im Projektordner unter target/criterion/report/ finden können.

Bitte kommentieren Sie Ihren Code in deutscher oder englischer Sprache um uns die Korrektur zu erleichtern.

## Bonus

Sie haben die Möglichkeit durch besondere Leistungen bis zu vier Bonuspunkte zu erhalten. Sie bekommen jeweils einen Bonuspunkt, falls Ihre Implementation funktioniert und:

- mehr als einen Produzenten gleichzeitig unterstützt,
- mehr als einen Konsumenten gleichzeitig unterstützt,
- der Sender nicht nur lockfrei, sondern sogar wartefrei umgesetzt ist,
- für den Anwendungsfall (ein Produzent, ein Konsument) asymptotisch schneller als die Implementation von MPSC-Kanälen aus der Standardbibliothek ist.

Bonuspunkte werden separat gezählt und können Ihre Punktzahl über das Maximum hinaus erhöhen. Nur (partiell) korrekte Implementationen qualifizieren sich für Bonuspunkte.

Ressourcen
Atomics (Rust Standardbibliothek)
MPSC (Rust Standardbibliothek)
Benchmarking mit Criterion
Lazy-Static
Abgabemodalitäten
Die Abgabe erfolgt als zip-Archiv über Moodle. Packen Sie das Cargo-Projekt und laden es bei Moodle hoch. Die Aufgabe ist 10+4 Punkte wert und der Abgabetermin ist in der Nacht vom 13. zum 14. November 2023 um 0 Uhr.
``
