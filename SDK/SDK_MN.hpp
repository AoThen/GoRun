#ifndef __SDK_MN_HPP__
#define __SDK_MN_HPP__

#include <string>


#ifndef FALSE
#define FALSE               0
#endif

#ifndef TRUE
#define TRUE                1
#endif


#define __MN_PLUGIN_IN__ extern "C" __declspec(dllexport)


//=======================================================================================
// 插件类型

#pragma region

#define _MN_PLUGIN_TYPE_SEARCH		100				// 插件类型-搜索


#pragma endregion


//=======================================================================================
// 事件

#pragma region

// 插件事件-被载入
//         param1: NULL
//         param2: NULL
//         return: 应返回 NULL
#define _MN_PLUGIN_EVENT_LOAD					1000

// 插件事件-销毁(被禁用不会调用此事件)
//         param1: NULL
//         param2: NULL
//         return: 应返回 NULL
#define _MN_PLUGIN_EVENT_DESTROY				9999

// 插件事件-被启用，当用户在 设置页 启用插件时，触发此事件
//         param1: NULL
//         param2: NULL
//         return: 应返回 NULL
#define _MN_PLUGIN_EVENT_ENABLE					2010

// 插件事件-被禁用，当用户在 设置页 禁用插件时，触发此事件
//         param1: NULL
//         param2: NULL
//         return: 应返回 NULL
#define _MN_PLUGIN_EVENT_DISABLE				2020

// 插件事件-配置被更改，当用户更改了设置时，触发此事件
//         param1: NULL
//         param2: NULL
//         return: 应返回 NULL
#define _MN_PLUGIN_EVENT_CONFIG_CHANGE			3000


// 插件事件-搜索-初始化，插件类型为 _MN_PLUGIN_TYPE_SEARCH 时，触发此事件，并要求设置callback触发条件信息
//         param1: 指向 _MN_PLUGIN_SEARCH_INFO 的指针
//         param2: NULL
//         return: 应返回 TRUE
#define _MN_PLUGIN_EVENT_SEARCH_INIT			5000






#pragma endregion


//=======================================================================================
// 搜索 Item - Action

#pragma region

#define _MN_PLUGIN_SEARCH_ITEM_ACTION_NIL        0        // 不执行任何操作
#define _MN_PLUGIN_SEARCH_ITEM_ACTION_COPY       101      // 复制                  [ val: param_0 (u8) ]
#define _MN_PLUGIN_SEARCH_ITEM_ACTION_NEWKEY     102      // 设置新的搜索关键词    [ val: param_0 (u8) ]
#define _MN_PLUGIN_SEARCH_ITEM_ACTION_CMD        103      // 执行 cmd 命令         [ val: param_0 (u8) ]
#define _MN_PLUGIN_SEARCH_ITEM_ACTION_RUN        104      // 执行 运行 命令        [ val: param_0 (u8) ]


#pragma endregion


//=======================================================================================


// Maye Nano - API
namespace mn {

    using _MN_RESULT   = long long;
    using _PFN_API_FN  = void* (*)(void* ctx, unsigned int opCode, void* p1, void* p2, void* p3, void* p4, void* p5);


    // 插件基本信息
    struct _MN_PLUGIN_INFO {

        const char*     name         = NULL;       // u8 名称
        const char*     description  = NULL;       // u8 描述

        const char*     version      = NULL;       // u8 版本
        const char*     author       = NULL;       // u8 作者
        const char*     email        = NULL;       // u8 电子信箱
        const char*     homepage     = NULL;       // u8 主页地址

        unsigned int    type         = NULL;       // 插件类型，_MN_PLUGIN_TYPE_***

        // 事件通知函数指针
        //     原型 _MN_RESULT _MN_Notify(unsigned int msg, void* param1, void* param2);
        //        msg:     消息，_MN_PLUGIN_EVENT_***
        //        param1:  参数1
        //        param2:  参数2
        //        return: 应返回 NULL
        void*            fnNotify    = NULL;

    };


    // 搜索 信息
    struct _MN_PLUGIN_SEARCH_INFO {
        const char*        name             = NULL;        // u8 名称
        const char*        desc             = NULL;        // u8 描述
        const char*        icon             = NULL;        //    图标(相对于插件目录)
        unsigned int       boost            = 0;           //    搜索结果权重(用于排序，数值越大越靠前)

        const char*        keys             = NULL;        // u8 关键字[以 \0 作为分隔 \0\0 作为结束符](可选，keys与regex必须保证设置其中一个)

        const char*        regex            = NULL;        //    正则匹配(可选，keys与regex必须保证设置其中一个)
        bool               isRegexMatch     = false;       //    正则匹配匹配模式，false=搜索子串匹配，true=全文精准匹配，

        bool               isGlobalResults  = false;       //    是否允许出现在全局结果内(性能影响)

        // 关键词变更执行函数
        //     原型 _MN_PLUGIN_SEARCH_RESULT_LIST _MN_Search_Callback(bool isClear, const char* prefix, const char* key, const char* key_low);
        //         isClear:  是否需要清空缓存。如果值为true，prefix,key,key_low参数值均为NULL，且需要清除上次搜索的缓存，进行释放内存
        //         prefix:   匹配成功的前缀，如果是正则匹配成功则参数值为空字符串
        //         key:      关键词[不包含前缀]
        //         key_low:  关键词[不包含前缀](小写)
        void*               fn = NULL;
    };

    // 搜索 结果 Item
    struct _MN_PLUGIN_SEARCH_RESULT_ITEM {
        unsigned int            boost       = 0;              //    搜索结果权重(用于排序，数值越大越靠前)

        bool                    isWordWrap  = false;          //    显示名称自动换行
        const char*             name        = NULL;           // u8 名称
        const char*             nameSub     = NULL;           // u8 名称-二级

        const char*             icon        = NULL;           //     图标(相对于插件目录)
        const unsigned char*    iconBuf     = NULL;           //     图标数据指针[标准图片](优先级高于icon)
        int                     iconBufLen  = NULL;           //     图标数据指针长度

        unsigned int            action      = NULL;           //     执行动作，_MN_PLUGIN_SEARCH_ITEM_ACTION_***
        const char*             param_0     = NULL;           //     参数 0
        void*                   param_1     = NULL;           //     参数 1
    };

    // 搜索 结果 List
    struct _MN_PLUGIN_SEARCH_RESULT_LIST {
        _MN_PLUGIN_SEARCH_RESULT_ITEM*   items = NULL;  // 指向 _MN_PLUGIN_SEARCH_RESULT_ITEM 的 Array
        unsigned int                     count = NULL;  // items Array 的数量
    };



    inline _PFN_API_FN  _MN_API_FN  = NULL;        // API Ptr
    inline void*        _MN_CTX     = NULL;        // Plugin Context


    void* __pluginApiCall(unsigned int opCode, void* param_1 = NULL, void* param_2 = NULL, void* param_3 = NULL, void* param_4 = NULL, void* param_5 = NULL) {
        if (!_MN_API_FN || !_MN_CTX) return NULL;
        return _MN_API_FN(_MN_CTX, opCode, param_1, param_2, param_3, param_4, param_5);
    }


    // 获取 Maye Nano 版本号
    // @return std::string 如:"1.0.0.1"
    std::string GetVersion() {
        std::string buf(50, '\0');
        size_t ws = (size_t)__pluginApiCall(50, buf.data(), (void*)buf.size());
        buf.resize(ws);
        return buf;
    }

    // 获取自身ID
    // @return std::string 如:"mn.cc.arae.hash"
    std::string GetSelfId() {
        std::string buf(512, '\0');
        size_t ws = (size_t)__pluginApiCall(100, buf.data(), (void*)buf.size());
        buf.resize(ws);
        return buf;
    }

    // 获取自身目录
    // @return std::string 如:".\plugin\mn.cc.arae.hash\"
    std::string GetSelfDir() {
        std::string buf(2048, '\0');
        size_t ws = (size_t)__pluginApiCall(110, buf.data(), (void*)buf.size());
        buf.resize(ws);
        return buf;
    }



    // 写配置项
    // @return bool
    bool SetCfgItem(const char* key, const char* val) {
        return (bool)__pluginApiCall(210, (void*)key, (void*)val);
    }

    // 读配置项
    // @param key
    // @param delVal 默认值
    // @param bufSize 读取缓冲区尺寸
    // @return std::string
    std::string GetCfgItem(const char* key, const char* delVal = NULL, size_t bufSize = 200) {
        std::string buf(bufSize, '\0');
        size_t ws = (size_t)__pluginApiCall(211, (void*)key, (void*)delVal, buf.data(), (void*)buf.size());
        buf.resize((std::min)((size_t)ws, buf.size()));
        return buf;
    }


    
}






#endif //__SDK_MN_HPP__
