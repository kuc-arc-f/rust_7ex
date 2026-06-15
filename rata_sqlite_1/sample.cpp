#include <iostream>
#include <cstring>
#include <string>
#include "include/MyTodo.hpp"

extern "C" {

    // message: Rustから送られてくる文字列
    // output_buffer: 戻り値を格納するバッファ（Rust側で用意）
    // buffer_size: バッファのサイズ
    __declspec(dllexport) int todo_add(
        const char* message,      // 入力文字列
        char* output_buffer,      // 出力バッファ
        int buffer_size           // バッファサイズ
    ) {
        int ret = 0;
        // 受信した文字列を表示
        //std::cout << "C++ Receive:" << message << std::endl;        
        std::string result = std::string(message);
        
        MyTodo todo_handler("");
        todo_handler.add_handler(result);

        // バッファサイズをチェックしてコピー
        if (result.length() < static_cast<size_t>(buffer_size)) {
            strcpy_s(output_buffer, buffer_size, result.c_str());
        } else {
            strcpy_s(output_buffer, buffer_size, "buffer too small");
            return -1;
        }
        return ret;
    }

    __declspec(dllexport) int todo_list(
        const char* message,      // 入力文字列
        char* output_buffer,      // 出力バッファ
        int buffer_size           // バッファサイズ
    ) {
        int ret = 0;
        
        MyTodo todo_handler("");
        std::string result = todo_handler.todo_list_handler();

        // バッファサイズをチェックしてコピー
        if (result.length() < static_cast<size_t>(buffer_size)) {
            strcpy_s(output_buffer, buffer_size, result.c_str());
        } else {
            strcpy_s(output_buffer, buffer_size, "buffer too small");
            return -1;
        }
        return ret;
    }

    __declspec(dllexport) int todo_delete(int id) {
        MyTodo todo_helper("");
        todo_helper.todo_delete_handler(id);
        return 1;
    } 

    // 整数の加算（既存）
    __declspec(dllexport) int add(int a, int b) {
        return a + b;
    }

    // メモリ解放用（必要に応じて）
    __declspec(dllexport) void free_string(char* ptr) {
        delete[] ptr;
    }
}