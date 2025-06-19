# \FileManagementApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_file_handler**](FileManagementApi.md#get_file_handler) | **GET** /api/v1/file/{hash} | Retrieve a file by its hash from path, with optional custom filename in request body.
[**upload_file_handler**](FileManagementApi.md#upload_file_handler) | **POST** /api/v1/file | Upload a file to the server.



## get_file_handler

> std::path::PathBuf get_file_handler(hash, file_download_request)
Retrieve a file by its hash from path, with optional custom filename in request body.

The file will be reconstructed from its blocks.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**hash** | **String** | File hash | [required] |
**file_download_request** | [**FileDownloadRequest**](FileDownloadRequest.md) | Optional custom filename for download | [required] |

### Return type

[**std::path::PathBuf**](std::path::PathBuf.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/octet-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## upload_file_handler

> models::FileUploadResponse upload_file_handler(body)
Upload a file to the server.

The file will be split into blocks and stored in the database.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**body** | **std::path::PathBuf** | File data to upload | [required] |

### Return type

[**models::FileUploadResponse**](FileUploadResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/octet-stream
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

