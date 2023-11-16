//! The module for game genie related code and structs.

/// The possible errors decoding a game genie code
pub enum GameGenieError {
    /// One or more of the characters in the code are not valid game genie characters.
    InvalidCodeCharacters,
    /// Invalid number of digits found in the game genie code
    InvalidLength,
}

/// Represents a valid game genie code
pub struct GameGenieCode {
    /// The code for an actual game genie
    code: String,
    /// The target address of the code
    target: u16,
    /// The replacement value for the code at the specified address.
    new_value: u8,
    /// The check value to compare the actual value with in order to activate the code.
    check_value: Option<u8>,
}

impl GameGenieCode {
    /// Convert a string reference to a game genie code, if possible.
    pub fn from_str(i: &str) -> Result<Self, GameGenieError> {
        if i.len() != 6 && i.len() != 8 {
            return Err(GameGenieError::InvalidLength);
        }
        for d in i.chars() {
            match d.to_ascii_uppercase() {
                'A' | 'E' | 'P' | 'O' | 'Z' | 'X' | 'L' | 'U' | 'G' | 'K' | 'I' | 'S' | 'T'
                | 'V' | 'Y' | 'N' => Ok(()),
                _ => Err(GameGenieError::InvalidCodeCharacters),
            }?;
        }
        let digits: Vec<u8> = i
            .chars()
            .map(|c| match c.to_ascii_uppercase() {
                'A' => 0,
                'E' => 8,
                'P' => 1,
                'O' => 9,
                'Z' => 2,
                'X' => 10,
                'L' => 3,
                'U' => 11,
                'G' => 4,
                'K' => 12,
                'I' => 5,
                'S' => 13,
                'T' => 6,
                'V' => 14,
                'Y' => 7,
                'N' => 15,
                _ => 0,
            })
            .collect();
        if digits.len() == 6 {
            let mut transposed: [u8; 6] = [0; 6];
            transposed[0] = (digits[2] & 8) | (digits[3] & 7);
            transposed[1] = (digits[4] & 8) | (digits[5] & 7);
            transposed[2] = (digits[1] & 8) | (digits[2] & 7);
            transposed[3] = (digits[3] & 8) | (digits[4] & 7);
            transposed[4] = (digits[0] & 8) | (digits[1] & 7);
            transposed[5] = (digits[5] & 8) | (digits[0] & 7);

            let address = 0x8000 | ((transposed[0] as u16) << 12)
                | ((transposed[1] as u16) << 8)
                | ((transposed[2] as u16) << 4)
                | (transposed[3] as u16);
            
            let data = ((transposed[4]) << 4)
            | transposed[5];
            return Ok(GameGenieCode {
                code: i.to_string(),
                target: address,
                new_value: data,
                check_value: None,
            });
        } else if digits.len() == 8 {
            let mut transposed: [u8; 8] = [0; 8];
            transposed[0] = (digits[2] & 8) | (digits[3] & 7);
            transposed[1] = (digits[4] & 8) | (digits[5] & 7);
            transposed[2] = (digits[1] & 8) | (digits[2] & 7);
            transposed[3] = (digits[3] & 8) | (digits[4] & 7);
            transposed[4] = (digits[0] & 8) | (digits[1] & 7);
            transposed[5] = (digits[7] & 8) | (digits[0] & 7);
            transposed[6] = (digits[6] & 8) | (digits[7] & 7);
            transposed[7] = (digits[5] & 8) | (digits[6] & 7);

            let address = 0x8000 | ((transposed[0] as u16) << 12)
                | ((transposed[1] as u16) << 8)
                | ((transposed[2] as u16) << 4)
                | (transposed[3] as u16);
            let data = ((transposed[4]) << 4)
                | transposed[5];
            let check = ((transposed[6]) << 4)
            | transposed[7];
            return Ok(GameGenieCode {
                code: i.to_string(),
                target: address,
                new_value: data,
                check_value: Some(check),
            });
        }
        Err(GameGenieError::InvalidCodeCharacters)
    }
}
