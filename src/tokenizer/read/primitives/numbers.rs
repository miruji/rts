use crate::tokenizer::types::token::{Token};
use crate::tokenizer::types::tokenType::TokenType;
// =================================================================================================

/// Проверяет что байт является цифрой
pub fn isDigit(byte: &u8) -> bool
{
  *byte >= b'0' && *byte <= b'9'
}

// =================================================================================================

/// Проверяет buffer по index и так находит возможные примитивные числовые типы данных;
/// `UInt, Int, UFloat, Float, Rational, Complex`
///
/// todo: Ввести Complex числа;
///
/// todo: Ввести работу float с .1 или . как 0.0
pub fn getNumber(buffer: &[u8], index: &mut usize, bufferLength: &usize) -> Token
{
  let mut savedIndex: usize = *index; // index buffer
  let mut result: String = String::from(buffer[savedIndex] as char);
  savedIndex += 1;

  let mut hasDot: bool = false; // dot check
  let mut negative: bool = false; // negative check
  let mut hasExponential: bool = false; // e, e+, e-

  let mut currentByte: u8; // Текущий символ
  while savedIndex < *bufferLength
  {
    currentByte = buffer[savedIndex]; // Значение текущего символа

    // Пропуск пустот
    if currentByte == b' ' || currentByte == b'\t'
    {
      savedIndex += 1;
      continue;
    }

    // todo: use match case
    if !negative && buffer[*index] == b'-'
    { // Int/Float
      // Логика тут простая - токенайзер должен вложить минус в число, потому что он рядом.
      // Любое алгебраическое выражение будь то `10-20` - всегда `-20` общая сущность.
      // Поэтому и раскрывается потом как: `10+(-20)`. Поэтому если минус перед числом -
      // он обязательно должен быть втянут в него. Поэтому для чисел он всегда унарный;
      // Для выражений: `a-b` он будет уже бинарный т.к. там логика парсера идет.
      result.push(currentByte as char);
      negative = true;
      savedIndex += 1;
    } else
    if isDigit(&currentByte)
    { // UInt
      result.push(currentByte as char);
      savedIndex += 1;
    } else
    if currentByte == b'.' && !hasDot
    { // UFloat
      
      // Нужно, чтобы читать: `12 34 . 20`
      let mut hasDigitAfterDot: bool = false;
      let mut lookAhead: usize = savedIndex +1;
      while lookAhead < *bufferLength
      {
        let nextByte: u8 = buffer[lookAhead];
        if nextByte == b' ' || nextByte == b'\t'
        {
          lookAhead += 1;
          continue;
        } else
        if isDigit(&nextByte)
        {
          hasDigitAfterDot = true;
        }
        break;
      }

      //
      if hasDigitAfterDot
      {
        hasDot = true;
        result.push(currentByte as char);
        savedIndex += 1;
      }
      else
      {
        break;
      }
    } else 
    if !hasExponential && (currentByte == b'e' || currentByte == b'E') 
    { // Это должно быть float, без повторений E.

      //
      hasExponential = true;
      result.push(currentByte as char);
      savedIndex += 1;
      hasDot = true; // Если будет integer - то станет от этого float

      // Нужно, чтобы читать: `12 34 e + 2`
      let mut lookAhead: usize = savedIndex;
      while lookAhead < *bufferLength
      {
        let nextByte: u8 = buffer[lookAhead];
        if nextByte == b' ' || nextByte == b'\t'
        {
          lookAhead += 1;
          continue;
        } else
        if nextByte == b'+' || nextByte == b'-'
        {
          result.push(nextByte as char);
          savedIndex = lookAhead + 1;
        }
        break;
      }
    } else { break; }
  }

  *index = savedIndex;

  // next return
  match (hasDot, negative)
  { // dot, negative
    (true, true)  => Token::new( TokenType::Float,    result ),
    (true, false) => Token::new( TokenType::UFloat,   result ),
    (false, true) => Token::new( TokenType::Int,      result ),
    _             => Token::new( TokenType::UInt,     result ),
  }
  //
}

// =================================================================================================

#[cfg(test)]
mod tests
{
  use crate::tokenizer::read::primitives::numbers::getNumber;
  use crate::tokenizer::types::token::{Token};
  use crate::tokenizer::types::tokenType::TokenType;
  // ===============================================================================================

  /// todo desk
  #[test]
  fn value() 
  {
    for (input, expectedType) in [
      // UInt
      ("0", TokenType::UInt),
      ("1", TokenType::UInt),
      ("1234567890", TokenType::UInt),

      // Int
      ("-0", TokenType::Int),
      ("-1", TokenType::Int),
      ("-987654321", TokenType::Int),

      // UFloat
      ("3.14", TokenType::UFloat),
      //("3.", TokenType::UFloat), // todo Сейчас нет реализации 3. = 3.0
      //(".14", TokenType::UFloat), // todo Сейчас нет реализации .14 = 0.14
      //(".", TokenType::UInt), // todo Сейчас нет реализации . = 0.0

      // UInt
      ("-14.3", TokenType::Float),
      ("-2.5", TokenType::Float),
      ("-100.1000", TokenType::Float),

      // Rational
      //("1//2", TokenType::Rational) // todo Rational пока что нет как типа
    ] {
      let buffer: &[u8] = input.as_bytes();
      let bufferLength: usize = buffer.len();
      let mut index: usize = 0;
      let token: Token = getNumber(buffer, &mut index, &bufferLength);

      //
      let tokenType: String = token.getDataType().to_string();
      let expectedType: String = expectedType.to_string();
      assert_eq!(
        tokenType,
        expectedType,
        "Для '{}' ожидался тип {}, получен {}",
        input,
        expectedType,
        tokenType
      );
      
      //
      let tokenData: String = token.getData().toString().unwrap_or_default();
      assert_eq!(
        tokenData,
        input,
        "Для '{}' значение '{}' не совпало",
        input,
        tokenData
      );
      
      //
      assert_eq!(index, bufferLength, "Индекс не дошел до конца для '{}'", input);
    }
    //
  }

  /// todo desk
  #[test]
  fn index()
  {
    for (input, expectedType, expectedValue, expectedIndex) in [
      ("123 ", TokenType::UInt, "123", 3),
      ("-42x", TokenType::Int, "-42", 3),
      ("3.14+", TokenType::UFloat, "3.14", 4),
      ("-5.5abc", TokenType::Float, "-5.5", 4),
      ("100500\n", TokenType::UInt, "100500", 6),
    ] {
      let buffer: &[u8] = input.as_bytes();
      let bufferLength: usize = buffer.len();
      let mut index: usize = 0;
      let token: Token = getNumber(buffer, &mut index, &bufferLength);

      //
      let tokenType: String = token.getDataType().to_string();
      let expectedType: String = expectedType.to_string();
      assert_eq!(
        tokenType,
        expectedType,
        "Для '{}' ожидался тип {}, получен {}",
        input,
        expectedType,
        tokenType
      );

      //
      let tokenData: String = token.getData().toString().unwrap_or_default();
      assert_eq!(
        tokenData,
        expectedValue,
        "Для '{}' ожидалось значение '{}', получено '{}'",
        input,
        expectedValue,
        tokenData
      );

      //
      assert_eq!(
        index, expectedIndex,
        "Для '{}' индекс должен остановиться на {}, а остановился на {}",
        input, expectedIndex, index
      );
    }
    //
  }

  // ===============================================================================================

  /// todo desk
  #[test]
  fn exponential() 
  {
    for (input, expectedType, expectedValue, expectedIndex) in [
      ("1e3", TokenType::UFloat, "1e3", 3),
      ("1.5e-2", TokenType::UFloat, "1.5e-2", 6),
      ("-3.14e+10", TokenType::Float, "-3.14e+10", 9),
      ("0e0", TokenType::UFloat, "0e0", 3),
      ("-1E-5", TokenType::Float, "-1E-5", 5),
      ("2e+5", TokenType::UFloat, "2e+5", 4),
      ("10e-1", TokenType::UFloat, "10e-1", 5),
    ] {
      let buffer: &[u8] = input.as_bytes();
      let bufferLength: usize = buffer.len();
      let mut index: usize = 0;

      let token: Token = getNumber(buffer, &mut index, &bufferLength);

      //
      let tokenType: String = token.getDataType().to_string();
      let expectedType: String = expectedType.to_string();

      assert_eq!(
        tokenType,
        expectedType,
        "Для '{}' ожидался тип {}, получен {}",
        input,
        expectedType,
        tokenType
      );

      //
      let tokenData: String = token.getData().toString().unwrap_or_default();

      assert_eq!(
        tokenData,
        expectedValue,
        "Для '{}' ожидалось значение '{}', получено '{}'",
        input,
        expectedValue,
        tokenData
      );

      //
      assert_eq!(
        index,
        expectedIndex,
        "Для '{}' индекс должен остановиться на {}, а остановился на {}",
        input,
        expectedIndex,
        index
      );
    }
  }

  // ===============================================================================================
}

// =================================================================================================