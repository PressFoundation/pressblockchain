// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice Cheap, attachable opinions/annotations on any article.
/// @dev Opinions are immutable once posted; no censorship/removal. Spam mitigated via small fee.
contract OpinionRegistry {
    event OpinionPosted(bytes32 indexed articleId, address indexed author, uint256 feePaid, string uri, string note);

    address public pressToken;
    address public treasury;
    uint256 public opinionFee; // in PRESS smallest units

    constructor(address _pressToken, address _treasury, uint256 _fee) {
        pressToken = _pressToken;
        treasury = _treasury;
        opinionFee = _fee;
    }

    function setOpinionFee(uint256 fee) external {
        // governance in later pass
        opinionFee = fee;
    }

    function postOpinion(bytes32 articleId, string calldata uri, string calldata note) external {
        require(articleId != bytes32(0), "BAD_ARTICLE");
        require(bytes(uri).length > 0 || bytes(note).length > 0, "EMPTY");
        if (opinionFee > 0) {
            (bool ok, ) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, treasury, opinionFee));
            require(ok, "FEE_FAIL");
        }
        emit OpinionPosted(articleId, msg.sender, opinionFee, uri, note);
    }
}
