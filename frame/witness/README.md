# Design

## links
https://gray-beach-094b4cc00.azurestaticapps.net/design/witness-design/

## data structure 

Workspace:
{
    "workspace_id": "uint256",
    "max_proposal_id": "uint256",
    "erc20_contract": "address",
    "additional_data": "bytes"
}

WorkspaceAdditionalData:
{
    "name": "String",
    "spec": "String",
    "contract": "address",
    "chainId": "uint32"
}

Proposal:
{
    "id": "uint256",
    "status": "uint256", // unnecessary to store it.
    "author": "address",
    "start": "uint64",  // block number
    "end": "uint64",    // block number
    "snapshot": "uint256", // block number in evm chain
    "data": "Bytes"   
}

ProposalData:
{
    "title": "String",
    "content": "String",
    "options": "Vec<String>", // how many options
    "options": "Vec<u32>", // how many votes
    // private, then just result. public, upate vote each time. medium 
    "privateLevel": "uint8", 
    "chainhooks": "Vec<CallbackInfo>" // call some functions in smart contract
}

CallbackInfo:
{
    "callback_type": "String", // solitity, ink or pallet
    "contract": "address", // where erc20, ink hash, or pallet id in automata
    "function_name": "String", // name
    "function_args": "Vec<String>",
    "function_vals": "Vec<String>"
}


## method
create workspace
create proposal
vote for proposal 

sudo only, remove workspace, remove proposal, set vote. create workspace, create proposal

hook. on finalize. check each proposal and try to finalized them. 

## parameter
max additional data len 
basic reserve fee
reserve fee per byte 
max workspace name
max workspace desc


max function name
max function args len
max function vals len 

## question 
1. reserve token?
2. origin priviledge
3. 