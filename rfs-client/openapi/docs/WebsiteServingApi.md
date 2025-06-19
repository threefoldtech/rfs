# \WebsiteServingApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**serve_website_handler**](WebsiteServingApi.md#serve_website_handler) | **GET** /api/v1/website/{website_hash}/{path} | 



## serve_website_handler

> std::path::PathBuf serve_website_handler(website_hash, path)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**website_hash** | **String** | flist hash of the website directory | [required] |
**path** | **String** | Path to the file within the website directory, defaults to index.html if empty | [required] |

### Return type

[**std::path::PathBuf**](std::path::PathBuf.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/octet-stream, application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

