"""
Thai Kedmanee Keyboard Layout Mapping

This module provides the standard Thai Kedmanee layout mapping from
QWERTY keyboard to Thai characters.
"""

# Thai Kedmanee layout mapping
# Maps QWERTY keys to Thai characters
KEDMANEE_KEYMAP = {
    # Numbers row (unshifted)
    '1': 'ๅ',  # THAI CHARACTER ANGKHANKHU
    '2': '/',  # Forward slash (commonly used in Thai)
    '3': '_',  # Underscore
    '4': 'ภ',  # THAI CHARACTER PHO PHUNG
    '5': 'ถ',  # THAI CHARACTER THO THUNG
    '6': 'ุ',  # THAI CHARACTER SARA U
    '7': 'ึ',  # THAI CHARACTER SARA UE
    '8': 'ค',  # THAI CHARACTER KHO KHWAI
    '9': 'ต',  # THAI CHARACTER TO TAO
    '0': 'จ',  # THAI CHARACTER CHO CHAN
    '-': 'ข',  # THAI CHARACTER KHO KHAI
    '=': 'ช',  # THAI CHARACTER CHO CHANG
    
    # Numbers row (shifted)
    '!': '+',  # Plus sign
    '@': '๑',  # THAI DIGIT ONE
    '#': '๒',  # THAI DIGIT TWO
    '$': '๓',  # THAI DIGIT THREE
    '%': '๔',  # THAI DIGIT FOUR
    '^': 'ู',  # THAI CHARACTER SARA UU
    '&': '฿',  # THAI CURRENCY SYMBOL BAHT
    '*': '๕',  # THAI DIGIT FIVE
    '(': '๖',  # THAI DIGIT SIX
    ')': '๗',  # THAI DIGIT SEVEN
    '_': '๘',  # THAI DIGIT EIGHT
    '+': '๙',  # THAI DIGIT NINE
    
    # Top row (QWERTY)
    'q': 'ๆ',  # THAI CHARACTER MAIYAMOK
    'w': 'ไ',  # THAI CHARACTER SARA AI MAIMALAI
    'e': 'ำ',  # THAI CHARACTER SARA AM
    'r': 'พ',  # THAI CHARACTER PHO PHAN
    't': 'ะ',  # THAI CHARACTER SARA A
    'y': 'ั',  # THAI CHARACTER MAI HAN-AKAT
    'u': 'ี',  # THAI CHARACTER SARA II
    'i': 'ร',  # THAI CHARACTER RO RUA
    'o': 'น',  # THAI CHARACTER NO NU
    'p': 'ย',  # THAI CHARACTER YO YAK
    '[': 'บ',  # THAI CHARACTER BO BAIMAI
    ']': 'ล',  # THAI CHARACTER LO LING
    
    # Top row (shifted)
    'Q': '๐',  # THAI DIGIT ZERO
    'W': '"',  # Quotation mark
    'E': 'ฎ',  # THAI CHARACTER DO CHADA
    'R': 'ฑ',  # THAI CHARACTER THO NANGMONTHO
    'T': 'ธ',  # THAI CHARACTER THO THONG
    'Y': 'ํ',  # THAI CHARACTER NIKHAHIT
    'U': '๊',  # THAI CHARACTER MAI TRI
    'I': 'ณ',  # THAI CHARACTER NO NEN
    'O': 'ฯ',  # THAI CHARACTER PAIYANNOI
    'P': 'ญ',  # THAI CHARACTER YO YING
    '{': 'ฐ',  # THAI CHARACTER THO THAN
    '}': ',',  # Comma
    
    # Middle row (ASDF)
    'a': 'ฟ',  # THAI CHARACTER FO FAN
    's': 'ห',  # THAI CHARACTER HO HIP
    'd': 'ก',  # THAI CHARACTER KO KAI
    'f': 'ด',  # THAI CHARACTER DO DEK
    'g': 'เ',  # THAI CHARACTER SARA E
    'h': '้',  # THAI CHARACTER MAI THO
    'j': '่',  # THAI CHARACTER MAI EK
    'k': 'า',  # THAI CHARACTER SARA AA
    'l': 'ส',  # THAI CHARACTER SO SUA
    ';': 'ว',  # THAI CHARACTER WO WAEN
    "'": 'ง',  # THAI CHARACTER NGO NGU
    
    # Middle row (shifted)
    'A': 'ฤ',  # THAI CHARACTER RU
    'S': 'ฆ',  # THAI CHARACTER KHO RAKHANG
    'D': 'ฏ',  # THAI CHARACTER TO PATAK
    'F': 'โ',  # THAI CHARACTER SARA O
    'G': 'ฌ',  # THAI CHARACTER CHO CHOE
    'H': '็',  # THAI CHARACTER MAITAIKHU
    'J': '๋',  # THAI CHARACTER MAI CHATTAWA
    'K': 'ษ',  # THAI CHARACTER SO RUSI
    'L': 'ศ',  # THAI CHARACTER SO SALA
    ':': 'ซ',  # THAI CHARACTER SO SO
    '"': '.',  # Period
    
    # Bottom row (ZXCV)
    'z': 'ผ',  # THAI CHARACTER PHO PHUNG
    'x': 'ป',  # THAI CHARACTER PO PLA
    'c': 'แ',  # THAI CHARACTER SARA AE
    'v': 'อ',  # THAI CHARACTER O ANG
    'b': 'ิ',  # THAI CHARACTER SARA I
    'n': 'ื',  # THAI CHARACTER SARA UE
    'm': 'ท',  # THAI CHARACTER THO THAHAN
    ',': 'ม',  # THAI CHARACTER MO MA
    '.': 'ใ',  # THAI CHARACTER SARA AI MAIMUAN
    '/': 'ฝ',  # THAI CHARACTER FO FA
    
    # Bottom row (shifted)
    'Z': '(',  # Left parenthesis
    'X': ')',  # Right parenthesis
    'C': 'ฉ',  # THAI CHARACTER CHO CHING
    'V': 'ฮ',  # THAI CHARACTER HO NOKHUK
    'B': 'ฺ',  # THAI CHARACTER PHINTHU
    'N': '์',  # THAI CHARACTER THANTHAKHAT
    'M': '?',  # Question mark
    '<': 'ฒ',  # THAI CHARACTER THO PHUTHAO
    '>': 'ฬ',  # THAI CHARACTER LO CHULA
    '?': 'ฦ',  # THAI CHARACTER LU
    
    # Space remains space
    ' ': ' ',
}

# Create reverse mapping for potential future use
REVERSE_KEDMANEE_KEYMAP = {v: k for k, v in KEDMANEE_KEYMAP.items()}

def qwerty_to_thai(text):
    """
    Convert QWERTY text to Thai using Kedmanee layout.
    
    Args:
        text (str): Input text in QWERTY
        
    Returns:
        str: Converted Thai text
    """
    result = []
    for char in text:
        thai_char = KEDMANEE_KEYMAP.get(char, char)
        result.append(thai_char)
    return ''.join(result)

def thai_to_qwerty(text):
    """
    Convert Thai text back to QWERTY using reverse Kedmanee layout.
    
    Args:
        text (str): Input Thai text
        
    Returns:
        str: Converted QWERTY text
    """
    result = []
    for char in text:
        qwerty_char = REVERSE_KEDMANEE_KEYMAP.get(char, char)
        result.append(qwerty_char)
    return ''.join(result)