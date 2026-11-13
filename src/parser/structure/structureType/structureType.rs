use crate::parser::bytes::Bytes;
use crate::tokenizer::types::token::Token;
use crate::tokenizer::types::tokenType::TokenType;
use serde::{Deserialize, Serialize};
use std::num::IntErrorKind;
// =================================================================================================

// Идея простая - т.к. мы имеем хранение в токенах, то это абстрактные данные;
// Вообще сам по себе токен бесконечен;
//
// Когда из `токенов` -> `линии` -> `блоки` цепочки появляется `Structure`, то он
// привязан к FFI уже, поэтому требует определенные физические типы;
//
// Внутри это все еще токены, но через `StructureType` - мы контролируем их.
//
//
// Поэтому нам следует "нормализовать" - т.е. привести к нужной форме 
// токены при хранении в `Structure`;
// 
// `StructureType` как раз указывает - как мы должны хранить токены и в каком виде.

// =================================================================================================

/// Вид примитива-значения.
/// 
/// Они также могут контактировать при перечислении через `|` уже в [`StructureType::Union`].
/// 
/// Для FFI их значения преобразовываются отдельно т.к. не являются типами-примитивами.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveValueKind
{
  /// Число;
  /// 
  /// `a: 10 = 10`.
  Number,
  
  /// Строка;
  /// 
  /// `a: "text" = "text"`.
  String
}

// =================================================================================================

macro_rules! defineEnum
{
  (
    $($(#[$varAttr:meta])* $variant:ident $(($field:ty))?),* $(,)?
  ) =>
  {
    /// Список структур-примитивов.
    /// 
    /// Он также включает в себя `Custom` - хвост для пользовательских типов;
    /// Хотя фактически они не примитивы - но они должны быть логически здесь;
    /// Поэтому из списков имен `Custom` исключен на этапе раскрытия макроса.
    #[repr(u8)]
    #[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
    pub enum PrimitiveType
    {
      $(
        $(#[$varAttr])*
        $variant $(($field))?,
      )*
    }

    impl PrimitiveType
    {
      /// Имена примитивов в порядке объявления (без `Custom`).
      const Names: &'static [&'static str] =
        defineEnum!(@names []; $($variant $(($field))?,)*);

      /// Пары `имя -> значение` для примитивов (без `Custom`).
      const Entries: &'static [(&'static str, PrimitiveType)] =
        defineEnum!(@entries []; $($variant $(($field))?,)*);

      /// Примитив по имени; `Custom` и неизвестные имена -> `None`.
      pub fn fromName(name: &str) -> Option<Self>
      {
        Self::Entries.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone())
      }

      /// Является ли строка именем примитива (не `Custom`).
      pub fn isPrimitive(name: &str) -> bool
      {
        Self::Names.contains(&name)
      }

      /// Перебор примитивов: `(имя, значение)`.
      pub fn iter() -> impl Iterator<Item = (&'static str, Self)>
      {
        Self::Entries.iter().cloned()
      }
    }
  };

  (@names [$($acc:tt)*];) =>
  {
    &[$($acc)*]
  };
  (@names [$($acc:tt)*]; $v:ident, $($rest:tt)*) =>
  {
    defineEnum!(@names [$($acc)* stringify!($v),]; $($rest)*)
  };
  (@names [$($acc:tt)*]; $v:ident ($t:ty), $($rest:tt)*) =>
  {
    defineEnum!(@names [$($acc)*]; $($rest)*)
  };

  (@entries [$($acc:tt)*];) =>
  {
    &[$($acc)*]
  };
  (@entries [$($acc:tt)*]; $v:ident, $($rest:tt)*) =>
  {
    defineEnum!(@entries
      [$($acc)* (stringify!($v), PrimitiveType::$v),];
      $($rest)*
    )
  };
  (@entries [$($acc:tt)*]; $v:ident ($t:ty), $($rest:tt)*) =>
  {
    defineEnum!(@entries [$($acc)*]; $($rest)*)
  };
}

defineEnum!(
  None,
  Any, // todo У неё нет использований, их надо потом будет добавить.
  
  True,
  False,

  U8, U16, U32, U64, Usize,
  I8, I16, I32, I64, Isize,
  F32, F64,

  /// Указатель (raw).
  Pointer,
  
  /// ABI-композит: 2 поля `.pointer` и `.length`.
  String,
  /// Алиас от `Pointer`.
  CString,
  /// Байты без длины/NUL в самом StructureType — длина берётся из токена при FFI-вызове.
  // todo rewrite desc.
  RawString,

  // todo Требует удаление для FFI-ABI?
  //  Это кстати может называться ещё `Block`, но не знаю точно пока что.
  //  Но это не мешает по идее FFI, а штука в целом нужная для RTS.
  //  Сама она не участвует в FFI.
  Method,
  // todo Требует удаление для FFI-ABI?
  //  Идея была сделать List<Type> но это под вопросом.
  List,
  
  // todo `Method` и `List` не понятно должны ли вообще сюда входить или быть в `StructureType`?
  //  Потому что это явно выше и не C ABI типы, а что-то более высокое внутри самого языка.
  //  Может быть если `Method` = `Module`? Может быть их можно объединить? Ведь это блоки кода.
  //  Но тоже как бы `None`/`Any` - тоже штуки языка, а не C ABI. Тут вопрос не в том, чтобы 
  //  сделать C ABI список - это есть в `bridge` для FFI, а в правильности нахождения абстракций.

  /// Позволяет создавать пользовательские типы.
  Custom(String)
);

// =================================================================================================

/// Тип данных структуры.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum StructureType
{
  /// Примитивные и пользовательские типы.
  /// 
  /// Формально пользовательские типы это не примитив, но они должны находиться в этой группе.
  Primitive(PrimitiveType),

  /// Примитив-значение как граница типизации: `a: 10 = 10`.
  ///
  /// Это ограничение значения - допустимо только точное совпадение.
  PrimitiveValue(PrimitiveValueKind, String),
  
  /// Объединение типов: `a: U8 | String = ...`.
  ///
  /// Хранится не "пересечение", а список допустимых вариантов,
  /// и в структуре единовременно лежит ровно один из них.
  ///
  /// **Семантика**:
  /// - При присваивании выбирается тот вариант, в который значение
  ///   помещается как есть, либо в который оно приводится;
  /// - Если ни один вариант не подходит и привести нельзя — `None`;
  /// - При полной мутабельности `~~` тип может меняться, поэтому объединение
  ///   можно указать, но оно ничего не ограничивает.
  ///
  /// Пустой список вариантов равносилен `None`.
  ///
  /// Вариантом может быть и [`StructureType::PrimitiveValue`]: `a: 10 | 20`, `a: "text" | U8`.
  Union(Vec<Self>),
  
  /// Модуль из `import(...)`; `id` — ключ lookup без имени структуры.
  ///
  /// Это правильно т.к. поиск по имени может уйти за структуру, а `id`
  /// позволяет остаться только в `self` scope.
  Module(usize)
}

// =================================================================================================

// todo Можно заменить данные на keywords из words.rs, 
//   но их не хватит т.к. тут есть другие, 
//   + здесь structure type
impl ToString for StructureType
{ // todo convert -> fmt::Display ?
  fn to_string(&self) -> String
  {
    match self
    { //
      Self::Primitive(PrimitiveType::None) => String::from("None"),
      Self::Primitive(PrimitiveType::Any) => String::from("Any"),
      
      //
      Self::Primitive(PrimitiveType::True) => String::from("True"),
      Self::Primitive(PrimitiveType::False) => String::from("False"),

      // Беззнаковые.
      Self::Primitive(PrimitiveType::U8) => String::from("U8"),
      Self::Primitive(PrimitiveType::U16) => String::from("U16"),
      Self::Primitive(PrimitiveType::U32) => String::from("U32"),
      Self::Primitive(PrimitiveType::U64) => String::from("U64"),
      // U128 нет т.к. это не FFI совместимый тип данных.
      Self::Primitive(PrimitiveType::Usize) => String::from("Usize"),

      // Знаковые.
      Self::Primitive(PrimitiveType::I8) => String::from("I8"),
      Self::Primitive(PrimitiveType::I16) => String::from("I16"),
      Self::Primitive(PrimitiveType::I32) => String::from("I32"),
      Self::Primitive(PrimitiveType::I64) => String::from("I64"),
      // I128 нет т.к. это не FFI совместимый тип данных.
      Self::Primitive(PrimitiveType::Isize) => String::from("Isize"),

      // Плавающие.
      // F16 нет т.к. это не FFI совместимый тип данных.
      Self::Primitive(PrimitiveType::F32) => String::from("F32"),
      Self::Primitive(PrimitiveType::F64) => String::from("F64"),

      // Указатель.
      Self::Primitive(PrimitiveType::Pointer) => String::from("Pointer"),
      // Алиас от `Pointer`, но для FFI - отдельный примитив (см. `abiType`).
      Self::Primitive(PrimitiveType::CString) => String::from("CString"),
      Self::Primitive(PrimitiveType::RawString) => String::from("RawString"),
      Self::Primitive(PrimitiveType::String) => String::from("String"),

      //
      Self::Primitive(PrimitiveType::Method) => String::from("Method"),
      Self::Primitive(PrimitiveType::List) => String::from("List"),

      // Пользовательский тип.
      Self::Primitive(PrimitiveType::Custom(value)) => value.clone(),

      // Объединение типов.
      Self::Union(variants) => variants
        .iter()
        .map(|variant: &Self| variant.to_string())
        .collect::<Vec<String>>()
        .join(" | "),

      // Примитив-значение.
      Self::PrimitiveValue(PrimitiveValueKind::Number, value) => value.clone(),
      Self::PrimitiveValue(PrimitiveValueKind::String, value) => format!("\"{}\"", value),

      // Module.
      Self::Module(_) => String::from("Module")
    }
  }
}

// =================================================================================================

/// В рамки приведения зажимается значение токена.
#[derive(Clone, Copy)]
enum CastFrame
{
  /// Границы целочисленного типа: (минимум, максимум).
  Integer(i64, u64),

  /// `f32`.
  Float32,

  /// `f64`.
  Float64
}

/// Значение токена - как число.
///
/// Читается по виду данных токена (`TokenType`): целое - точно, `float` - с дробью.
///
/// У целых два числа: точное значение и величина из данных токена.
enum Number
{
  /// Беззнаковое значение -> в рамку.
  ///
  /// `text` - данные токена: для `float` величина берётся из них, а не из
  /// уже насыщенного `u64` - токен бесконечен, и число может быть больше.
  Unsigned(u64, f64),

  /// Знаковое значение -> в рамку.
  ///
  /// `text` - данные токена: для `float` величина берётся из них, а не из
  /// уже насыщенного `i64` - токен бесконечен, и число может быть больше.
  Signed(i64, f64),

  /// Значение с плавающей точкой -> в рамку.
  Float(f64)
}

impl Number
{
  /// Величина из данных токена.
  fn magnitude(&self) -> f64
  {
    match *self
    {
      Number::Unsigned(_, magnitude) => magnitude,
      Number::Signed(_, magnitude) => magnitude,
      Number::Float(value) => value,
    }
  }
}

impl CastFrame
{
  /// Положить число в рамку.
  ///
  /// `number` - прочитанное по виду данных токена число.
  fn setNumber(&self, token: &mut Token, number: Number) -> ()
  {
    match *self
    {
      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      
      CastFrame::Integer(min, max) => match number
      {
        Number::Unsigned(value, _) =>
          token.setData( Bytes::from(value.min(max).to_string()) ),
        //
        Number::Signed(value, _) =>
        {
          let normalized: String =
            if value < min {
              min.to_string()
            } else if value > 0 && (value as u64) > max {
              max.to_string()
            } else {
              value.to_string()
            };
          token.setData( Bytes::from(normalized) );
        }
        //
        Number::Float(value) =>
        {
          // Приведение к целочисленным типам: округление и зажим в границы типа;
          //
          // Отрицательное значение остаётся, если тип его принимает (`I8 = -5.5` -> `-6`),
          // а для беззнаковых типов оно становится 0 (`U8 = -5.5` -> `0`).
          let rounded: f64 = value.round();
          let normalized: String = if rounded < 0.0
          { // Приведение `f64` в `i64` насыщается само, `inf` тоже.
            // todo Тут упомянут `inf` - нужно разобраться т.к. его нет в RTS.
            (rounded as i64).max(min).to_string()
          } else
          { // todo Тут бы desc как сделано в ветке сверху?
            (rounded as u64).min(max).to_string()
          };
          token.setData( Bytes::from(normalized) );
        }
      }
      
      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      
      CastFrame::Float32 =>
      {
        let floatValue: f64 = number.magnitude().clamp(f32::MIN as f64, f32::MAX as f64);
        // `{:?}` для `f32`/`f64` сохраняет точку для целых `float`:
        // `1.0` -> `1.0`, не `1`; и использует `e`-нотацию только для
        // больших/маленьких чисел. Так `data` всегда одинаково проходит 
        // через токенайзер как `Float`/`UFloat`.
        token.setData( Bytes::from(format!("{:?}", floatValue as f32)) );
      }
      
      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      
      CastFrame::Float64 =>
      {
        let floatValue: f64 = number.magnitude().clamp(f64::MIN, f64::MAX);
        // `{:?}` для `f32`/`f64` сохраняет точку для целых `float`:
        // `1.0` -> `1.0`, не `1`; и использует `e`-нотацию только для
        // больших/маленьких чисел. Так `data` всегда одинаково проходит 
        // через токенайзер как `Float`/`UFloat`.
        token.setData( Bytes::from(format!("{:?}", floatValue)) );
      }
      
      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    }
  }
}

// =================================================================================================

impl StructureType
{
  /// ABI / storage тип: примитив сводится к natural-форме значения.
  ///
  /// `Primitive(String, _)` → `String`; `Primitive(Number, v)` → наименьший числовой
  /// ABI-тип для `v`; `Primitive(CString)` → `Pointer` (алиас); всё остальное — как есть.
  /// 
  /// Нужен там, где `dataType` участвует в FFI, string-полях и `castToken`, а не в `stype`.
  /// 
  /// todo Я бы изменил название и desc и так далее. Потому что слишком непонятная штука.
  pub fn abiType(&self) -> Self
  {
    match self
    {
      //
      Self::PrimitiveValue(PrimitiveValueKind::String, _) => Self::Primitive(PrimitiveType::String),
      // `CString` - алиас от `Pointer`: в ABI-форме это указатель.
      Self::Primitive(PrimitiveType::CString) => Self::Primitive(PrimitiveType::Pointer),
      //
      Self::PrimitiveValue(PrimitiveValueKind::Number, value) =>
      {
        // Те же правила, что naturalType для числового токена: через временный UInt/Int.
        let tokenType: TokenType = 
          if value.starts_with('-') {
            TokenType::Int
          } else
          if value.contains('.') {
            TokenType::UFloat
          } else {
            TokenType::UInt
          };
        let mut token: Token = Token::new(tokenType, value.clone());
        Self::naturalType(&mut token)
      }
      other => other.clone()
    }
  }

  /// Тип, который значение занимает САМО по себе, без учёта объявления.
  ///
  /// Для числа это наименьший ABI-тип, в который оно помещается
  /// (и насыщение по границам самого широкого типа, #71);
  /// для строки — `String`. Токен при этом НЕ очищается,
  /// в отличие от `Token::getStructureType()`, который чистит всё нечисловое.
  pub fn naturalType(token: &mut Token) -> Self
  {
    match token.getDataType()
    {
      TokenType::None => Self::Primitive(PrimitiveType::None),
      TokenType::Any => Self::Primitive(PrimitiveType::Any),
      TokenType::True => Self::Primitive(PrimitiveType::True),
      TokenType::False => Self::Primitive(PrimitiveType::False),
      TokenType::String => Self::Primitive(PrimitiveType::String),
      TokenType::RawString => Self::Primitive(PrimitiveType::RawString),
      // Числа: ширину и насыщение считает сам getStructureType.
      TokenType::UInt | TokenType::Int | TokenType::UFloat | TokenType::Float =>
        token.getStructureType(),
      // Неизвестное — трактуем как отсутствие значения.
      _ => Self::Primitive(PrimitiveType::None)
    }
  }

  /// Выдаёт `TokenType`, соответствующий текущему `StructureType`:
  ///
  /// - `U8`/`U16`/`U32`/`U64`/`Usize` ⟶ `UInt`;
  /// 
  /// - `I8`/`I16`/`I32`/`I64`/`Isize` ⟶ `Int`;
  /// 
  /// - `F32`/`F64` ⟶ `UFloat` для неотрицательного значения, иначе `Float`;
  /// 
  /// - Прочие примитивы ⟶ `None`, тип токена не пересчитывается.
  fn getTokenType(&self, tokenValue: &str) -> Option<TokenType>
  {
    match self.castFrame()
    {
      // Целочисленный тип: знак - по нижней границе.
      Some(CastFrame::Integer(min, _)) =>
        Some(if min < 0 { TokenType::Int } else { TokenType::UInt }),
      //
      Some(CastFrame::Float32) | Some(CastFrame::Float64) =>
        Some(if tokenValue.starts_with('-') { TokenType::Float } else { TokenType::UFloat }),
      //
      None => None
    }
  }

  // ===============================================================================================
  
  /// Возвращает рамку границ для приведения текущего типа;
  /// Удобно, когда надо знать минимум и максимум.
  fn castFrame(&self) -> Option<CastFrame>
  {
    match self
    {
      //
      StructureType::Primitive(PrimitiveType::U8)    => Some(CastFrame::Integer(0,                 u8::MAX    as u64)),
      StructureType::Primitive(PrimitiveType::U16)   => Some(CastFrame::Integer(0,                 u16::MAX   as u64)),
      StructureType::Primitive(PrimitiveType::U32)   => Some(CastFrame::Integer(0,                 u32::MAX   as u64)),
      StructureType::Primitive(PrimitiveType::U64)   => Some(CastFrame::Integer(0,                 u64::MAX         )),
      StructureType::Primitive(PrimitiveType::Usize) => Some(CastFrame::Integer(0,                 usize::MAX as u64)),
      //
      StructureType::Primitive(PrimitiveType::I8)    => Some(CastFrame::Integer(i8::MIN    as i64, i8::MAX    as u64)),
      StructureType::Primitive(PrimitiveType::I16)   => Some(CastFrame::Integer(i16::MIN   as i64, i16::MAX   as u64)),
      StructureType::Primitive(PrimitiveType::I32)   => Some(CastFrame::Integer(i32::MIN   as i64, i32::MAX   as u64)),
      StructureType::Primitive(PrimitiveType::I64)   => Some(CastFrame::Integer(i64::MIN,          i64::MAX   as u64)),
      StructureType::Primitive(PrimitiveType::Isize) => Some(CastFrame::Integer(isize::MIN as i64, isize::MAX as u64)),
      //
      StructureType::Primitive(PrimitiveType::F32)   => Some(CastFrame::Float32),
      StructureType::Primitive(PrimitiveType::F64)   => Some(CastFrame::Float64),
      //
      _ => None
    }
  }

  // ===============================================================================================

  /// Приводит токен в рамки [`StructureType`], чтобы [`Structure`] смог его хранить.
  ///
  /// **Нужно для:**
  /// - Методы приведения `U8(a)`;
  /// - Типизация параметров `abc(a: U8)`;
  /// - Объявление `a: U8 := ...`.
  ///
  //  todo Я бы переписал всё что ниже по desc или удалил.
  /// **Важно**:
  ///
  /// - `Union` - вариант подбирается по значению, приведение к нему `normalizeUnion`;
  ///
  /// - `None` - это отсутствие значения, а не значение: токен очищается, как у `None(a)`;
  ///
  /// - `String`/`RawString` - значение становится строкой этого типа:
  ///   `String(5)` - это `"5"`;
  ///
  /// - Остальное приводится по границам `StructureType`: число зажимается в
  ///   границы типа, значение не того рода становится базовым значением типа
  ///   (`U8(-10)` -> `0`, как и `U8("test")` -> `0`).
  ///
  /// - Границы приведения живут в рамке ([`CastFrame`]): она есть только у
  ///   числовых типов; у остальных приведения нет - значение проходит как есть.
  ///
  /// - `TokenType` и `StructureType` - разные вещи: `TokenType` - вид данных
  ///   в токене (по нему значение читается), `StructureType` - рамка, в которую
  ///   значение зажимается.
  //
  // todo Есть идея ввести на уровне `Structure` в будущем `bool`,
  //  чтобы можно было проверить и оптимизировать эту работу.
  pub fn castToken(&self, token: &mut Token) -> ()
  {
    // ---------------------------------------------------------------------------------------------

    // `None` - отсутствие значения: `abc(a: None)` и `None(a)` очищают токен.
    if self == &StructureType::Primitive(PrimitiveType::None)
    {
      token.setDataType(TokenType::None);
      token.setData(None);
      return;
    }

    // ---------------------------------------------------------------------------------------------
    
    // Строки: значение становится строкой этого типа, как `String(5)` - `"5"`.
    if matches!(
      self,
      StructureType::Primitive(PrimitiveType::String) |
      StructureType::Primitive(PrimitiveType::RawString)
    )
    {
      let data: String = token.getData().toString().unwrap_or_default();
      token.setDataType(
        if *self == StructureType::Primitive(PrimitiveType::String) {
          TokenType::String
        } else {
          TokenType::RawString
        }
      );
      token.setData(data);
      return;
    }

    // ---------------------------------------------------------------------------------------------
    
    // Замыкание для перерасчёта `TokenType`.
    let retypeToken = |token: &mut Token, structureType: &StructureType| -> ()
    {
      let tokenValue: String = token.getData().toString().unwrap_or_default();
      if let Some(targetTokenType) = structureType.getTokenType(&tokenValue) {
        token.setDataType(targetTokenType);
      }
    };

    // ---------------------------------------------------------------------------------------------
    
    // Получаем строку из данных.
    let tokenData: String =
      if let Some(tokenData) = token.getData().toString() {
        tokenData
      } else
      { // Нет данных.
        self.setTokenDefault(token);
        retypeToken(token, self);
        return;
      };

    //
    let dataType: &TokenType = token.getDataType();

    // ---------------------------------------------------------------------------------------------
    
    // Обработка типов.
    match self.castFrame()
    {
      // Здесь на самом деле мог бы быть `StructureType` в `match`,
      // но легче проверять через `TokenType`.
      Some(frame) => match dataType
      {
        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

        TokenType::UInt =>
        {
          // Токен бесконечен, а `u64` нет: поэтому всё что больше - граница `u64`,
          // дальше значение зажимается в рамки структуры.
          let value: Option<u64> = match tokenData.parse::<u64>()
          {
            Ok(value) => Some(value),
            Err(error) if *error.kind() == IntErrorKind::PosOverflow => Some(u64::MAX),
            Err(_) => None
          };

          match value
          {
            None => self.setTokenDefault(token), // Не распарсилось — базовое значение.
            Some(value) =>
            {
              // Для `float` берём величину числа напрямую, без сжатия в `u64`.
              let magnitude: f64 = tokenData.parse::<f64>().ok().unwrap_or(value as f64);
              frame.setNumber(token, Number::Unsigned(value, magnitude))
            }
          }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

        TokenType::Int =>
        {
          // Токен бесконечен, а `i64` нет: всё что больше или меньше - граница `i64`,
          // дальше значение зажимается в рамки структуры.
          let value: Option<i64> = match tokenData.parse::<i64>()
          {
            Ok(value) => Some(value),
            Err(error) if *error.kind() == IntErrorKind::NegOverflow => Some(i64::MIN),
            Err(error) if *error.kind() == IntErrorKind::PosOverflow => Some(i64::MAX),
            Err(_) => None
          };

          match value
          {
            None => self.setTokenDefault(token), // Не распарсилось — базовое значение.
            Some(value) =>
            {
              // Для `float` берём величину числа напрямую, без сжатия в `i64`.
              let magnitude: f64 = tokenData.parse::<f64>().ok().unwrap_or(value as f64);
              frame.setNumber(token, Number::Signed(value, magnitude))
            }
          }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

        TokenType::Float | TokenType::UFloat =>
        {
          if let Ok(mut value) = tokenData.parse::<f64>()
          {
            // Для `UFloat` обрезаем отрицательные до `0`.
            if dataType == &TokenType::UFloat && value < 0.0 {
              value = 0.0;
            }

            if value.is_nan()
            { // `NaN` — базовое значение;
              //
              // Бесконечность зажмётся в границы типа.
              self.setTokenDefault(token);
              retypeToken(token, self);
              return;
            }

            frame.setNumber(token, Number::Float(value));
          } else
          { // Не распарсилось — базовое значение.
            self.setTokenDefault(token);
          }
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

        _ =>
        { // Здесь пытаются прировнять что-то не то.
          self.setTokenDefault(token);
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      }
      None => {
        // Другие типы — ничего не делаем.
        //
        // todo По сути здесь должна быть логика рамок для других типов;
        //  Это всё вытекает из `константного поведения`;
        //  Но пока что здесь ничего нет; Возможно и не надо будет.
      }
    }

    // ---------------------------------------------------------------------------------------------

    // Пересчёт `TokenType`.
    retypeToken(token, self);

    // ---------------------------------------------------------------------------------------------
  }

  // ===============================================================================================
  
  /// Ставит стандартное значение токена при указании требуемого [`StructureType`];
  /// 
  /// Очень удобно для случаев, если в структуру приравнивают что-то не то,
  /// и нам надо `default` значение.
  /// 
  /// Например, если `a: U8 = "test"`, то будет `0` из-за константного поведения.
  fn setTokenDefault(&self, token: &mut Token) -> ()
  {
    // Создаём базовый токен.
    match self
    {
      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      
      // Целочисленные типы.
      StructureType::Primitive(PrimitiveType::U8) | StructureType::Primitive(PrimitiveType::U16) |
      StructureType::Primitive(PrimitiveType::U32) | StructureType::Primitive(PrimitiveType::U64) |
      StructureType::Primitive(PrimitiveType::Usize) |
      //
      StructureType::Primitive(PrimitiveType::I8) | StructureType::Primitive(PrimitiveType::I16) |
      StructureType::Primitive(PrimitiveType::I32) | StructureType::Primitive(PrimitiveType::I64) |
      StructureType::Primitive(PrimitiveType::Isize) =>
      {
        token.setDataType(TokenType::UInt);
        token.setData("0");
      }

      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      
      // Числа с плавающей точкой.
      StructureType::Primitive(PrimitiveType::F32) | StructureType::Primitive(PrimitiveType::F64) =>
      {
        token.setDataType(TokenType::Float); // todo Почему не UFloat? по сути же он должен быть как UInt выше сделан.
        token.setData("0.0");
      }

      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      
      StructureType::Primitive(PrimitiveType::True) =>
      {
        token.setDataType(TokenType::True);
        token.setData("True");
      }
      StructureType::Primitive(PrimitiveType::False) =>
      {
        token.setDataType(TokenType::False);
        token.setData("False");
      }

      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
      
      // Для остальных типов - ничего.
      //
      // todo Другие типы нужны тут?
      _ =>
      {
        token.setDataType(TokenType::None);
        token.setData(None);
      }

      // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    };
  }

  // ===============================================================================================
}

// =================================================================================================

impl Token
{
  // ===============================================================================================
  
  /// Вычисляет [`StructureType`] на основе токена.
  /// 
  /// Удобно, когда нет рамок структуры (не указан её тип) и нужно понять, 
  /// что в неё положили, но при этом в рамках [`StructureType`].
  /// 
  /// Если станет `None` - то токен будет очищен.
  /// 
  /// todo Было бы хорошо убрать как-то mut отсюда.
  pub fn getStructureType(&mut self) -> StructureType
  {
    let result = |selfToken: &mut Self, structureType: StructureType| -> StructureType
    {
      if structureType == StructureType::Primitive(PrimitiveType::None) {
        selfToken.setData(None);
      }
      structureType
    };
    
    //
    let dataType: &TokenType = self.getDataType();
    
    // Строки: без этого значение без аннотации (`a := "text"`) получало `None`,
    // а токен очищался.
    // 
    // Поля `pointer` и `length` тоже зависят от этого типа.
    //
    // Идёт до чтения данных: у пустой строки `""` данных нет, но это всё равно `String`.
    match dataType
    {
      TokenType::String => return StructureType::Primitive(PrimitiveType::String),
      TokenType::RawString => return StructureType::Primitive(PrimitiveType::RawString),
      _ => {}
    }

    // Получаем строку из данных токена.
    let data: String = if let Some(string) = self.getData().toString() {
      string
    } else {
      return result(self, StructureType::Primitive(PrimitiveType::None))
    };

    // Токены `UInt`/`Int`/`UFloat`/`Float` бесконечны, а ABI типы структур ограничены;
    //
    // Если число вышло за самый крайний ABI тип, то это константное поведение:
    // тип становится крайним (`U64`, `I64`, `F64`), а значение - его границей;
    //
    // Здесь хранится новое значение, если оно изменилось.
    let mut newData: Option<String> = None;

    let structureType: StructureType = match dataType 
    {
      TokenType::None => StructureType::Primitive(PrimitiveType::None),
      TokenType::Any => StructureType::Primitive(PrimitiveType::Any),
      //
      TokenType::True => StructureType::Primitive(PrimitiveType::True),
      TokenType::False => StructureType::Primitive(PrimitiveType::False),
      // Ссылка - не значение, а указатель на структуру: своего типа у неё нет,
      // но данные сохраняются - ссылка должна остаться ссылкой.
      TokenType::Link => return StructureType::Primitive(PrimitiveType::None),
      //
      TokenType::UInt => 
      {
        match data.parse::<u64>() 
        {
          Ok(value) if value <= u8::MAX  as u64 => StructureType::Primitive(PrimitiveType::U8),
          Ok(value) if value <= u16::MAX as u64 => StructureType::Primitive(PrimitiveType::U16),
          Ok(value) if value <= u32::MAX as u64 => StructureType::Primitive(PrimitiveType::U32),
          Ok(_) => StructureType::Primitive(PrimitiveType::U64),
          // Больше `U64`: вид ошибки говорит, что это переполнение числа.
          Err(error) if *error.kind() == IntErrorKind::PosOverflow => 
          {
            newData = Some( u64::MAX.to_string() );
            StructureType::Primitive(PrimitiveType::U64)
          }
          // Что-то непонятное.
          Err(_) => StructureType::Primitive(PrimitiveType::None)
        }
      }
      TokenType::Int => 
      {
        match data.parse::<i64>() 
        {
          Ok(value) if value >= i8::MIN  as i64 && value <= i8::MAX  as i64 => StructureType::Primitive(PrimitiveType::I8),
          Ok(value) if value >= i16::MIN as i64 && value <= i16::MAX as i64 => StructureType::Primitive(PrimitiveType::I16),
          Ok(value) if value >= i32::MIN as i64 && value <= i32::MAX as i64 => StructureType::Primitive(PrimitiveType::I32),
          Ok(_) => StructureType::Primitive(PrimitiveType::I64),
          // Меньше или больше `I64`: вид ошибки говорит, что это переполнение числа.
          Err(error) => match error.kind() 
          {
            IntErrorKind::NegOverflow => 
            {
              newData = Some( i64::MIN.to_string() );
              StructureType::Primitive(PrimitiveType::I64)
            }
            IntErrorKind::PosOverflow => 
            {
              newData = Some( i64::MAX.to_string() );
              StructureType::Primitive(PrimitiveType::I64)
            }
            // Что-то непонятное.
            _ => StructureType::Primitive(PrimitiveType::None)
          }
        }
      }
      //
      TokenType::UFloat | TokenType::Float => 
      {
        match data.parse::<f64>() 
        {
          Ok(value) if value.is_nan() => StructureType::Primitive(PrimitiveType::None),
          Ok(value) if value >= f32::MIN as f64 && value <= f32::MAX as f64 => StructureType::Primitive(PrimitiveType::F32),
          Ok(value) => 
          { // Самый крайний тип; `inf` становится границей `F64`.
            if value.is_infinite() 
            {
              let border: f64 = if value.is_sign_negative() { f64::MIN } else { f64::MAX };
              newData = Some( format!("{:e}", border) );
            }
            StructureType::Primitive(PrimitiveType::F64)
          }
          // Что-то непонятное.
          Err(_) => StructureType::Primitive(PrimitiveType::None)
        }
      }
      // Для остальных типов - `None`, и токен очищается.
      //
      // todo Сейчас могут попасть лишние т.к. они не объявлены выше.
      _ => StructureType::Primitive(PrimitiveType::None)
    };

    if let Some(newData) = newData {
      self.setData( Bytes::from(newData) );
    }
    result(self, structureType)
    //
  }

  /// Вычисляет `[StructureType`] на основе токена;
  /// 
  /// Но делает это упрощенно по имени токена - т.е. когда мы явно знаем уже тип в строке.
  /// 
  /// Например: токен хранит `U64` -> получаем `StructureType::U64`.
  pub fn getStructureTypeSimple(&self) -> StructureType
  {
    let tokenData: String = self.to_string();
    let structureTypeName: &str = tokenData.as_str();
    
    if PrimitiveType::isPrimitive(structureTypeName) {
      StructureType::Primitive(
        PrimitiveType::fromName(structureTypeName).unwrap() // Здесь точно есть т.к. проверено `is` выше.
      )
    } else
    { // Всё остальное — `custom`.
      StructureType::Primitive(PrimitiveType::Custom(tokenData))
    }
    //
  }

  // ===============================================================================================
}

// =================================================================================================
