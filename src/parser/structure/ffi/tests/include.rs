use crate::parser::parser::runCodeFile;
use crate::parser::structure::structure::Structure;
use crate::parser::structure::structureType::StructureType;
use crate::parser::testing::{checkStructure, writeCodeDir, writeCodeFileTo};
use std::sync::{Arc, RwLock};
// =================================================================================================

// Тесты на `import()`: путь считается от файла, в котором он написан.

// =================================================================================================

/// `import("./a.rt")` — простой импорт одного файла.
#[test]
fn simple() -> ()
{
  let dir: String = writeCodeDir!();

  let aPath: String = writeCodeFileTo!(
    &dir,
    r#"
    hello() {
      := 42
    }

    value: Usize = 4
    "#,
    "a.rt"
  );

  let mainPath: String = writeCodeFileTo!(
    &dir,
    r#"
    a := import("./a.rt")
    "#,
    "main.rt"
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&mainPath);

  checkStructure!(main, "a.hello", stype StructureType::Method);
  checkStructure!(main, "a.value", stype StructureType::Usize, value "4");

  let _ = std::fs::remove_file(&aPath);
  let _ = std::fs::remove_file(&mainPath);
  let _ = std::fs::remove_dir(&dir);
}

/// Вложенный `import()`: путь считается от файла, где он написан, а не от main.
#[test]
#[ignore]
fn nested() -> ()
{
  let dir: String = writeCodeDir!();
  let libDir: String = format!("{dir}/lib");
  let deepDir: String = format!("{libDir}/deep");
  std::fs::create_dir_all(&libDir).unwrap();
  std::fs::create_dir_all(&deepDir).unwrap();

  // lib/b.rt — рядом с a.rt.
  let bPath: String = writeCodeFileTo!(
    &libDir,
    r#"
    hello() {
      := "b: hello"
    }
    "#,
    "b.rt"
  );

  // lib/a.rt — импортирует b.rt из своей папки.
  let aPath: String = writeCodeFileTo!(
    &libDir,
    r#"
    b := import("./b.rt")
    bHello := b.hello()

    hello() {
      := "a: hello"
    }

    load() {
      c := import("./deep/c.rt")
      := c.hello()
    }
    "#,
    "a.rt"
  );

  // lib/deep/d.rt — рядом с c.rt.
  let dPath: String = writeCodeFileTo!(
    &deepDir,
    r#"
    hello() {
      := "d: hello"
    }
    "#,
    "d.rt"
  );

  // lib/deep/c.rt — импортирует d.rt из своей папки.
  let cPath: String = writeCodeFileTo!(
    &deepDir,
    r#"
    d := import("./d.rt")
    dHello := d.hello()

    hello() {
      := "c: hello"
    }
    "#,
    "c.rt"
  );

  // main.rt — импортирует a.rt из lib/.
  let mainPath: String = writeCodeFileTo!(
    &dir,
    r#"
    a := import("./lib/a.rt")
    c := import("./lib/deep/c.rt")
    "#,
    "main.rt"
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&mainPath);

  // a.rt импортировал b.rt правильно (путь от a.rt, не от main.rt).
  checkStructure!(main, "a.bHello", value "b: hello");
  checkStructure!(main, "a.hello", stype StructureType::Method);

  // c.rt импортировал d.rt правильно (путь от c.rt, не от main.rt).
  checkStructure!(main, "c.dHello", value "d: hello");
  checkStructure!(main, "c.hello", stype StructureType::Method);

  let _ = std::fs::remove_file(&bPath);
  let _ = std::fs::remove_file(&aPath);
  let _ = std::fs::remove_file(&dPath);
  let _ = std::fs::remove_file(&cPath);
  let _ = std::fs::remove_file(&mainPath);
  let _ = std::fs::remove_dir(&deepDir);
  let _ = std::fs::remove_dir(&libDir);
  let _ = std::fs::remove_dir(&dir);
}

// =================================================================================================
