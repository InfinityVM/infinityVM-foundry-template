// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

library Utils {
    function uintToString(uint v) internal pure returns (string memory str) {
        uint maxlength = 100;
        bytes memory reversed = new bytes(maxlength);
        uint i = 0;
        while (v != 0) {
            uint remainder = v % 10;
            v = v / 10;
            reversed[i++] = bytes1(uint8(48 + remainder));
        }
        bytes memory s = new bytes(i); // i holds the length of the number
        for (uint j = 0; j < i; j++) {
            s[j] = reversed[i - j - 1]; // reverse the string
        }
        str = string(s); // memory isn't implicitly convertible to storage
    }

    ///@dev converts bytes array to its ASCII hex string representation
    function toHexString(bytes memory input) public pure returns (string memory) {
        require(input.length < type(uint256).max / 2 - 1);
        bytes16 symbols = "0123456789abcdef";
        bytes memory hex_buffer = new bytes(2 * input.length + 2);
        hex_buffer[0] = "0";
        hex_buffer[1] = "x";

        uint256 pos = 2;
        uint256 length = input.length;
        for (uint256 i = 0; i < length; ++i) {
            uint256 _byte = uint8(input[i]);
            hex_buffer[pos++] = symbols[_byte >> 4];
            hex_buffer[pos++] = symbols[_byte & 0xf];
        }
        return string(hex_buffer);
    }
}
