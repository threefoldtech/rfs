# \BlockManagementApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**check_block_handler**](BlockManagementApi.md#check_block_handler) | **HEAD** /api/v1/block/{hash} | Checks a block by its hash.
[**get_block_downloads_handler**](BlockManagementApi.md#get_block_downloads_handler) | **GET** /api/v1/block/{hash}/downloads | Retrieve the number of times a block has been downloaded.
[**get_block_handler**](BlockManagementApi.md#get_block_handler) | **GET** /api/v1/block/{hash} | Retrieve a block by its hash.
[**get_blocks_by_hash_handler**](BlockManagementApi.md#get_blocks_by_hash_handler) | **GET** /api/v1/blocks/{hash} | Retrieve blocks by hash (file hash or block hash).
[**get_user_blocks_handler**](BlockManagementApi.md#get_user_blocks_handler) | **GET** /api/v1/user/blocks | Retrieve all blocks uploaded by a specific user.
[**list_blocks_handler**](BlockManagementApi.md#list_blocks_handler) | **GET** /api/v1/blocks | List all block hashes in the server with pagination
[**upload_block_handler**](BlockManagementApi.md#upload_block_handler) | **POST** /api/v1/block | Upload a block to the server.
[**verify_blocks_handler**](BlockManagementApi.md#verify_blocks_handler) | **POST** /api/v1/block/verify | Verify if multiple blocks exist on the server.



## check_block_handler

> check_block_handler(hash)
Checks a block by its hash.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**hash** | **String** | Block hash | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_block_downloads_handler

> models::BlockDownloadsResponse get_block_downloads_handler(hash)
Retrieve the number of times a block has been downloaded.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**hash** | **String** | Block hash | [required] |

### Return type

[**models::BlockDownloadsResponse**](BlockDownloadsResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_block_handler

> std::path::PathBuf get_block_handler(hash)
Retrieve a block by its hash.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**hash** | **String** | Block hash | [required] |

### Return type

[**std::path::PathBuf**](std::path::PathBuf.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/octet-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_blocks_by_hash_handler

> models::BlocksResponse get_blocks_by_hash_handler(hash)
Retrieve blocks by hash (file hash or block hash).

If the hash is a file hash, returns all blocks with their block index related to that file. If the hash is a block hash, returns the block itself.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**hash** | **String** | File hash or block hash | [required] |

### Return type

[**models::BlocksResponse**](BlocksResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_user_blocks_handler

> models::UserBlocksResponse get_user_blocks_handler(page, per_page)
Retrieve all blocks uploaded by a specific user.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | Page number (1-indexed) |  |
**per_page** | Option<**i32**> | Number of items per page |  |

### Return type

[**models::UserBlocksResponse**](UserBlocksResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_blocks_handler

> models::ListBlocksResponse list_blocks_handler(page, per_page)
List all block hashes in the server with pagination

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | Page number (1-indexed) |  |
**per_page** | Option<**i32**> | Number of items per page |  |

### Return type

[**models::ListBlocksResponse**](ListBlocksResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## upload_block_handler

> models::BlockUploadedResponse upload_block_handler(file_hash, idx, body)
Upload a block to the server.

If the block already exists, the server will return a 200 OK response. If the block is new, the server will return a 201 Created response.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**file_hash** | **String** | File hash associated with the block | [required] |
**idx** | **i64** | Block index within the file | [required] |
**body** | **std::path::PathBuf** | Block data to upload | [required] |

### Return type

[**models::BlockUploadedResponse**](BlockUploadedResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/octet-stream
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## verify_blocks_handler

> models::VerifyBlocksResponse verify_blocks_handler(verify_blocks_request)
Verify if multiple blocks exist on the server.

Returns a list of missing blocks.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**verify_blocks_request** | [**VerifyBlocksRequest**](VerifyBlocksRequest.md) | List of block hashes to verify | [required] |

### Return type

[**models::VerifyBlocksResponse**](VerifyBlocksResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

