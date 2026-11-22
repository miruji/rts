use crate::parser::runCodeFile;
use crate::parser::structure::structure::Structure;
use crate::parser::testing::{buildLibrary, checkStructure, writeCodeFile};
use std::sync::{Arc, RwLock};
// =================================================================================================

// Тесты на FFI `Structure`.
// Здесь проверяются именные и безымянные структуры.
//
// Результат FFI-вызова пишется в переменную, объявленную выше структуры:
// так видно, что структура держит свой `scope` и может менять внешние структуры.

// =================================================================================================

/// Хелпер: Собирает библиотеку с функцией `hello` для принятия и возврата числа.
fn buildHelloLibrary() -> String
{
  buildLibrary!(
    r#"
    int hello(int number) {
      return number;
    }
    "#
  )
}

// =================================================================================================

/// 1: Просто за структурой;
/// FFI вызов автоматически обернётся в структуру.
#[test]
fn plainCall() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    lib: Pointer = importNative("{libraryPath}")
    result = lib.hello(1)
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);

  checkStructure!(main, "result", stype U32, value "1");

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

/// 2: Перед методом.
#[test]
fn methodStructure() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    [ffi]
    test() {{
      lib: Pointer = importNative("{libraryPath}")
      result = lib.hello(2)
    }}
    test()
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);

  checkStructure!(main, "result", stype U32, value "2");

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

/// 3: Перед методом (в одну строку).
#[test]
fn methodStructureOneLine() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    [ffi] test2() {{
      lib: Pointer = importNative("{libraryPath}")
      result = lib.hello(3)
    }}
    test2()
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);

  checkStructure!(main, "result", stype U32, value "3");

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

/// 4: Безымянная структура.
#[test]
fn unnamedStructure() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    [ffi]
    {{
      lib: Pointer = importNative("{libraryPath}")
      result = lib.hello(4)
    }}
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);

  checkStructure!(main, "result", stype U32, value "4");

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

/// 5: Безымянная структура (в одну строку).
#[test]
fn unnamedStructureOneLine() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    [ffi] {{
      lib: Pointer = importNative("{libraryPath}")
      result = lib.hello(5)
    }}
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);

  checkStructure!(main, "result", stype U32, value "5");

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

/// 6: Именованная структура.
#[test]
fn namedStructure() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    [ffi]
    name {{
      lib: Pointer = importNative("{libraryPath}")
      result = lib.hello(6)
    }}
    
    name()
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);
  
  checkStructure!(main, "result", stype U32, value "6");
  checkStructure!(main, "name", stype Method);

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

/// 7: Именованная структура (в одну строку).
#[test]
fn namedStructureOneLine() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    [ffi] name {{
      lib: Pointer = importNative("{libraryPath}")
      result = lib.hello(7)
    }}
    
    name()
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);
  
  checkStructure!(main, "result", stype U32, value "7");
  checkStructure!(main, "name", stype Method);

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

/// 8: Тег с именем на строке, а структура — на следующей строке.
///
/// Тег не должен "висеть" на структуре ниже: токен на имя может быть только один.
/// Структура ниже — обычная вложенная структура, без FFI тега.
#[test]
fn incompleteFfiTag() -> ()
{
  let libraryPath: String = buildHelloLibrary();

  let path: String = writeCodeFile!(
    r#"
    result~: U32 = 0

    [ffi] name
    plain {{
      lib: Pointer = importNative("{libraryPath}")
      result = lib.hello(8)
    }}
    "#
  );

  let main: Arc<RwLock<Structure>> = runCodeFile(&path);

  // Структура ниже — создана, как обычная вложенная структура.
  checkStructure!(main, "plain", stype Method);
  // Но это не FFI-структура: тело не исполняется.
  checkStructure!(main, "result", stype U32, value "0");
  // Тег с именем, но без тела — не структура.
  assert!(main.read().unwrap().getStructureByScope("name").is_none());

  let _ = std::fs::remove_file(&path);
  let _ = std::fs::remove_file(&libraryPath);
}

// =================================================================================================
