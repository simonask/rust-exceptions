#include <exception>
#include <string>
#include <cassert>


struct FakeTraitObject {
    void* p0;
    void* p1;
};

struct NativeCppException {
    virtual ~NativeCppException() {}
    virtual const char* what() = 0;
    virtual std::exception_ptr get_exception_ptr() const = 0;
};

struct RustExceptionAsCppException : NativeCppException {
    RustExceptionAsCppException(FakeTraitObject ex) : exception(ex) {}

    FakeTraitObject exception;

    const char* what() final {
        return "<rust exception>";
    }

    std::exception_ptr get_exception_ptr() const {
        assert(false && "Expected a C++ exception, but was a Rust exception.");
    }
};

struct UnknownException : NativeCppException {
    UnknownException(std::exception_ptr ex) : exception(ex) {}

    std::exception_ptr exception;

    const char* what() final {
        return "<unknown exception>";
    }

    std::exception_ptr get_exception_ptr() const {
        return exception;
    }
};

struct StandardException : NativeCppException {
    std::exception_ptr ptr;
    std::string message;

    StandardException(std::exception_ptr ptr, std::exception& ex) : ptr(ptr), message(ex.what()) {}

    const char* what() final {
        return message.c_str();
    }

    std::exception_ptr get_exception_ptr() const {
        return ptr;
    }
};


extern "C"
void
cpp_exception_destroy(void* exception) {
    auto ex = reinterpret_cast<NativeCppException*>(exception);
    delete ex;
}


extern "C"
FakeTraitObject
cpp_try(void(*try_block)(void*), void* state, bool* caught_rust) {
    FakeTraitObject fto = {0};
    try {
        try_block(state);
    }
    catch (FakeTraitObject& exception) {
        *caught_rust = true;
        fto = exception;
    }
    catch (std::exception& exception) {
        *caught_rust = false;
        fto.p0 = new StandardException{std::current_exception(), exception};
    }
    catch (...) {
        *caught_rust = false;
        fto.p0 = new UnknownException{std::current_exception()};
    }
    return fto;
}


extern "C"
void
cpp_throw_rust(FakeTraitObject fto) {
    throw fto;
}


extern "C"
void
cpp_rethrow(void* exception) {
    auto ex = reinterpret_cast<NativeCppException*>(exception);
    auto exptr = ex->get_exception_ptr();
    std::rethrow_exception(exptr);
}


extern "C"
const char*
cpp_exception_what(void* exception) {
    auto ex = reinterpret_cast<NativeCppException*>(exception);
    return ex->what();
}


struct TestException : std::exception {
    std::string msg;
    TestException(std::string msg) : msg(std::move(msg)) {}

    const char* what() const noexcept override {
        return msg.c_str();
    }
};

extern "C"
void
cpp_throw_test_exception(const char* message) {
    throw TestException(message);
}

