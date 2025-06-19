# \FlistManagementApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_flist_handler**](FlistManagementApi.md#create_flist_handler) | **POST** /api/v1/fl | 
[**get_flist_state_handler**](FlistManagementApi.md#get_flist_state_handler) | **GET** /api/v1/fl/{job_id} | 
[**list_flists_handler**](FlistManagementApi.md#list_flists_handler) | **GET** /api/v1/fl | 
[**preview_flist_handler**](FlistManagementApi.md#preview_flist_handler) | **GET** /api/v1/fl/preview/{flist_path} | 



## create_flist_handler

> models::Job create_flist_handler(flist_body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**flist_body** | [**FlistBody**](FlistBody.md) |  | [required] |

### Return type

[**models::Job**](Job.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_flist_state_handler

> models::FlistStateResponse get_flist_state_handler(job_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**job_id** | **String** | flist job id | [required] |

### Return type

[**models::FlistStateResponse**](FlistStateResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_flists_handler

> std::collections::HashMap<String, Vec<models::FileInfo>> list_flists_handler()


### Parameters

This endpoint does not need any parameter.

### Return type

[**std::collections::HashMap<String, Vec<models::FileInfo>>**](Vec.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## preview_flist_handler

> models::PreviewResponse preview_flist_handler(flist_path)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**flist_path** | **String** | flist file path | [required] |

### Return type

[**models::PreviewResponse**](PreviewResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

