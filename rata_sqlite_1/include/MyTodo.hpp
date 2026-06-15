#pragma once
#include <windows.h>
#include <algorithm>
#include <ctime>
#include <shellapi.h>    // CommandLineToArgvW
#include <fcntl.h>       // _O_U16TEXT
#include <io.h>          // _setmode
#include <iostream>
#include <fstream>
#include <vector>
#include <string>
#include "nlohmann/json.hpp"

#include "MyDb.hpp"

#pragma comment(lib, "shell32.lib")

using json = nlohmann::json;

const std::string DB_PATH = "todo.db";
const std::string FILE_PATH = "todos.json";

/*
struct Todo {
    int id;
    std::string title;
};
*/

struct TodoData {
    int max_id;
    std::vector<Todo> items;
};

class MyTodo {
private:
    std::string m_name;

public:
    explicit MyTodo(std::string str){
        // ① stdout をワイド文字モードに切り替え
        _setmode(_fileno(stdout), _O_U16TEXT);

        // ② コンソールの出力コードページを UTF-8 に設定
        SetConsoleOutputCP(CP_UTF8);
        SetConsoleCP(CP_UTF8);            
    }

    ~MyTodo() {}


    // JSONファイルからデータ読み込み
    TodoData load_data() {
        TodoData data{0, {}};
        std::ifstream file(FILE_PATH);
        if (file.is_open()) {
            try {
                json j = json::parse(file);
                if (j.contains("max_id") && j["max_id"].is_number()) {
                    data.max_id = j["max_id"].get<int>();
                }
                if (j.contains("items") && j["items"].is_array()) {
                    for (const auto& item : j["items"]) {
                        if (item.contains("id") && item.contains("title")) {
                            data.items.push_back({item["id"].get<int>(), item["title"].get<std::string>()});
                        }
                    }
                }
            } catch (const json::parse_error& e) {
                std::cerr << "JSON解析エラー: " << e.what() << "\n";
            }
        }
        return data;
    }

    // データをJSONファイルに保存
    void save_data(const TodoData& data) {
        json j;
        j["max_id"] = data.max_id;
        j["items"] = json::array();
        for (const auto& item : data.items) {
            j["items"].push_back({{"id", item.id}, {"title", item.title}});
        }
        std::ofstream file(FILE_PATH);
        if (file.is_open()) {
            file << j.dump(2); // インデント2で整形出力
            file.close();
        } else {
            std::cerr << "エラー: ファイルに書き込めません。\n";
        }
    }

    // TODO追加
    void add_todo(TodoData& data, const std::string& title) {
        data.max_id++;
        data.items.push_back({data.max_id, title});
        save_data(data);
    }

    // TODO一覧表示
    void list_todos(const TodoData& data) {
        if (data.items.empty()) {
            std::cout << "TODO none\n";
            return;
        }
        for (const auto& item : data.items) {
            //std::cout << "[" << item.id << "] " << item.title << "\n";
            std::wcout << L"[" << item.id << L"] " << StringToWString(item.title) << L"\n";
        }
    } 

    // TODO削除
    void delete_todo(TodoData& data, int id) {
        auto it = std::remove_if(data.items.begin(), data.items.end(),
                                [id](const Todo& t) { return t.id == id; });
        if (it == data.items.end()) {
            //std::cout << "ID #" << id << " none \n";
            std::wcout << L"ID #" << id << L" none \n";
        } else {
            data.items.erase(it, data.items.end());
            save_data(data);
            //std::cout << "delete: #" << id << "\n";
            std::wcout << L"delete: #" << id << L"\n";
        }
    }

    void print_help() {
        std::cout << "使用方法: todo <コマンド> [引数]\n"
                << "コマンド:\n"
                << "  add <タイトル>      TODOを追加（複数単語可）\n"
                << "  list                TODO一覧を表示\n"
                << "  delete <ID>         IDを指定して削除\n"
                << "  help                ヘルプを表示\n";
    }

    void add_handler(std::string title){
        MyDb db_helper(DB_PATH);
        db_helper.add(title);
    }

    std::string todo_list_handler(){
        std::string ret = "";
        try{
            MyDb db_helper(DB_PATH);
            auto todos = db_helper.list("all");
            std::string ret = db_helper.list_json(todos);
            return ret;
        } catch (const std::exception& e) {
            std::cout << "Error , main" << std::endl;
            return ret;
        }  
    }

    void todo_delete_handler(int id){
        try{
            MyDb db_helper(DB_PATH);
            db_helper.remove(id);
        } catch (const std::exception& e) {
            std::cout << "Error , main" << std::endl;
        }  
    }
};

