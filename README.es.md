<p align="center">
  <img alt="Cerne" src="assets/cerne-banner-light.svg#gh-light-mode-only" width="400">
  <img alt="Cerne" src="assets/cerne-banner-dark.svg#gh-dark-mode-only" width="400">
</p>

<p align="center">
  🇺🇸 <a href="README.en.md">English</a> ·
  🇧🇷 <a href="README.pt-BR.md">Português</a> ·
  🇪🇸 Español
</p>

<br>

Un sistema no se vuelve legado porque alguien lo escribió mal. Se vuelve legado porque, a lo largo de los años, recibe miles de cambios que tienen sentido uno a uno: un acuerdo sindical, una ley nueva, una decisión judicial, un impuesto que cambió. Cada equipo resuelve su ticket como puede, muchas veces de madrugada, con un `if` en medio del código "solo por ahora". La regla antigua sigue funcionando al lado de la nueva, porque nadie se atreve a tocarla. Como resume el canal gotoCobol:

> "La entropía nace de la acumulación de soluciones localmente justificables, pero que nunca se reorganizan como un todo." (traducido del portugués)

Sistemas así se construyen todos los días, y la cuenta llega años después, a la hora de modernizar. Entonces no basta con cambiar de lenguaje: las reglas de negocio están dispersas, sin nombre y sin historia, y hay que desenterrar cada una antes de cambiar nada.

Cerne no impide que el sistema cambie. Le da a cada cambio un lugar adecuado y un nombre:

- **Toda regla tiene nombre y dirección.** Una decisión judicial se convierte en una `BusinessRule` con su frase, en la sección `Business rules` del command al que afecta, y no en un `if` perdido en medio del código. Quien llega después encuentra la regla por su nombre, y cuando rechaza una petición, el error dice cuál fue.
- **El código tiene la forma del tablero.** Cada post-it del Event Storming es un tipo de Rust, y todo `execute` tiene las mismas secciones, en el mismo orden. Equipos diferentes, en años diferentes, pueden escribir con el mismo formato, porque `cerne g` les da a todos el mismo esqueleto.
- **El tablero y el código cuentan la misma historia.** La conversación con el negocio ocurre en el tablero, con las mismas palabras que el código. Una regla nueva empieza como un post-it y termina en el lugar que indica el post-it.
- **Una reacción es una policy, no un efecto secundario.** "Siempre que X, haz Y" se convierte en una `Policy` con nombre, y el event bus ejecuta su command. Nadie tiene que buscar dónde fue a parar la reacción.

Esa es la propuesta de Cerne: un sistema que dure 30 años y siga siendo legible, no porque nada haya cambiado, sino porque cada cambio quedó a la vista.

Inspirado en el video [Entropia de software não é a mesma coisa que complexidade estrutural](https://www.youtube.com/watch?v=wTQbXp86m78) (en portugués), del canal gotoCobol.

## ¿Qué es Cerne?

Cerne es un framework Rust que convierte un tablero de [Event Storming](https://www.eventstorming.com) en código: cada post-it se convierte en un tipo tuyo que implementa un trait de Cerne, y cada flujo se convierte en un `execute` con las secciones del tablero.

Para que la aplicación mantenga esa forma mientras crece, Cerne viene con un CLI, `cerne`. `cerne new` crea el proyecto ya organizado como el tablero, y cada `cerne g` genera la pieza nueva (un command, un evento, una entidad, un port) en el lugar correcto, con las secciones correctas y ya compilando. Quien llega al proyecto no tiene que adivinar dónde vive la feature nueva ni cómo escribirla: ya nace dentro de la propuesta.

Entender el Event Storming es la clave para entender Cerne. El tablero cuenta una historia con post-its de colores: un actor envía un command (azul), un agregado (amarillo) cambia, ocurre un evento de dominio (naranja), una policy (lila) reacciona con un nuevo command, se llama a sistemas externos (rosa), y los read models (verde) muestran el resultado. Cerne divide la aplicación en tres capas: Domain, Application e Infrastructure.

### Capa Domain

La capa Domain es el corazón del tablero: entidades y agregados (`Entity`, `Aggregate`), value objects (`ValueObject`), eventos de dominio (`DomainEvent`) y las policies que reaccionan a ellos (`Policy`). Es pura y síncrona: aquí no ocurre ningún IO. Las invariantes (`Invariant`) son reglas con nombre sobre lo que siempre es cierto, y cuando una se rompe, el error enumera los nombres tal como están escritos en el tablero: `["quantity is positive"]`.

### Capa Application

La capa Application es donde actúan los actores. Un command (`Command`) abre su transacción, lee los ports, comprueba las reglas de negocio (`BusinessRule`), cambia un agregado y guarda sus eventos en el event outbox (`EventOutbox`), siempre en ese orden, así que el `execute` se lee como un flujo del tablero. Una query (`Query`) devuelve un read model, un struct de campos simples. Los ports son traits asíncronos para los repositorios y los sistemas externos. Quien envió el command publica sus eventos en un event bus, que ejecuta las policies: el `SequentialEventBus`, una cadena a la vez, o el `ConcurrentEventBus`, todos los eventos a la vez. El bus espera lo mejor: una policy que falla va a su `on_error`, y cómo sobrevive cada policy a un fallo lo decide la aplicación.

### Capa Infrastructure

Cerne es, ante todo, un framework para modelar el dominio y la aplicación, no la infraestructura, y no trae adapters. Los ports son traits: los repositorios, el event outbox y cada sistema externo los implementa la aplicación, sobre la base de datos, el framework web y la cola que elija. `cerne new` escribe una función con un `todo!()` donde va cada adapter.

## Crates

- [`cerne`](https://crates.io/crates/cerne): la biblioteca.
- [`cerne-cli`](https://crates.io/crates/cerne-cli): el comando `cerne`, que crea un proyecto organizado como el tablero (`cerne new`) y genera cada post-it en su lugar, ya compilando (`cerne g`).
- [`cerne-macros`](https://crates.io/crates/cerne-macros): los atributos `#[entity]`, `#[aggregate]` y `#[value_object]`. `cerne::domain` los reexporta, así que un proyecto solo depende de `cerne`.

## Primeros pasos

1. Instala el comando `cerne` (Rust 1.88 o más reciente):

   ```bash
   cargo install cerne-cli
   ```

2. Crea un proyecto:

   ```bash
   cerne new shop
   ```

3. Genera un agregado y un command, y ejecuta los tests:

   ```bash
   cd shop
   cerne g entity Order product:String quantity:u32 --aggregate
   cerne g command PlaceOrder product:String quantity:u32
   cargo test
   ```

4. Rellena los post-its. Estos recursos te ayudarán:
   - [El tutorial](docs/es/tutorial.md): una tienda, del tablero al código, con todos los post-its.
   - [La documentación de la API](https://docs.rs/cerne)
   - `cerne` sin argumentos enumera todos los generators.

## Contribuir

Las contribuciones son bienvenidas. [CONTRIBUTING.md](CONTRIBUTING.md) explica cómo ejecutar las comprobaciones y el estilo de código. Todo cambio que rompe la compatibilidad va al [CHANGELOG](CHANGELOG.md).

## Licencia

Cerne se distribuye bajo la [Apache License 2.0](LICENSE). El código que generan `cerne new` y `cerne g` pertenece a quien lo generó, que puede usarlo bajo cualquier licencia.
