pragma solidity ^0.4.24;

import "./Verifier.sol";
import "./VerificationKeys.sol";


contract PlasmaStub is VerificationKeys {

    uint32 constant DEADLINE = 3600; // seconds, to define

    event BlockCommitted(uint32 indexed blockNumber);
    event BlockVerified(uint32 indexed blockNumber);

    enum Circuit {
        DEPOSIT,
        UPDATE,
        WITHDRAWAL
    }

    struct Block {
        Circuit circuit;

        uint128 totalFees;
        bytes32 newRoot;
        bytes32 finalHash;

        // TODO: Everybody should be able to provide proof and collect fees when deadline is crossed
        address prover;
        uint32  deadline;
    }

    bytes32 public lastVerifiedRoot;

    // Key is block number
    mapping (uint32 => Block) public blocks;

    uint32 public totalCommitted;
    uint32 public totalVerified;

    // Balances for distributing fees to provers
    mapping (address => uint256) public balance;

    // Public API

    constructor() public {
        lastVerifiedRoot = EMPTY_TREE_ROOT;
    }
    
    function commitBlock(uint32 blockNumber, uint128 totalFees, bytes memory txDataPacked, bytes32 newRoot) public {
        require(blockNumber == totalCommitted + 1, "may only commit next block");

        bytes32 finalHash = createPublicDataCommitment(blockNumber, totalFees, txDataPacked);

        // // TODO: need a strategy to avoid front-running msg.sender
        blocks[blockNumber] = Block(Circuit.UPDATE, totalFees, newRoot, finalHash, msg.sender, uint32(block.timestamp + DEADLINE));
        emit BlockCommitted(blockNumber);
        totalCommitted++;
    }

    function createPublicDataCommitment(uint32 blockNumber, uint128 totalFees, bytes memory txDataPacked)
    public 
    pure
    returns (bytes32 h) {

        bytes32 initialHash = sha256(abi.encodePacked(uint256(blockNumber), uint256(totalFees)));
        bytes32 finalHash = sha256(abi.encodePacked(initialHash, txDataPacked));
        
        return finalHash;
    }

    function verifyBlock(uint32 blockNumber, uint256[8] memory proof) public {
        require(totalVerified < totalCommitted, "no committed block to verify");
        require(blockNumber == totalVerified + 1, "may only verify next block");
        Block memory committed = blocks[blockNumber];
        bool verification_success = verifyUpdateProof(proof, lastVerifiedRoot, committed.newRoot, committed.finalHash);
        require(verification_success, "invalid proof");

        emit BlockVerified(blockNumber);
        totalVerified++;
        lastVerifiedRoot = committed.newRoot;

        // TODO: how to deal with deadline? Penalties?
        balance[committed.prover] += committed.totalFees;
    }

    function verifyUpdateProof(uint256[8] memory, bytes32, bytes32, bytes32) internal view returns (bool valid);

}


contract Plasma is PlasmaStub, Verifier {
    // Implementation

    function verifyUpdateProof(uint256[8] memory proof, bytes32 oldRoot, bytes32 newRoot, bytes32 finalHash)
        internal view returns (bool valid)
    {
        uint256 mask = (~uint256(0)) >> 3;
        uint256[14] memory vk;
        uint256[] memory gammaABC;
        (vk, gammaABC) = getVkUpdateCircuit();
        uint256[] memory inputs = new uint256[](3);
        inputs[0] = uint256(oldRoot);
        inputs[1] = uint256(newRoot);
        inputs[2] = uint256(finalHash) & mask;
        return Verify(vk, gammaABC, proof, inputs);
    }

}

contract PlasmaTest is PlasmaStub, Verifier {
    function verifyUpdateProof(uint256[8] memory proof, bytes32 oldRoot, bytes32 newRoot, bytes32 finalHash)
        internal view returns (bool valid)
    {
        // always approve for test
        return true;
    }

}